package desktop

import (
	"archive/tar"
	"archive/zip"
	"bytes"
	"compress/gzip"
	"context"
	"crypto/sha256"
	"fmt"
	"io"
	"net/http"
	"os"
	"runtime"
	"strings"
	"testing"
)

type roundTripFunc func(*http.Request) (*http.Response, error)

func (f roundTripFunc) RoundTrip(r *http.Request) (*http.Response, error) { return f(r) }
func TestKernelDownloadIntegrityAndPreservation(t *testing.T) {
	k := helperKernel(t)
	original, e := os.ReadFile(k.Binary())
	if e != nil {
		t.Fatal(e)
	}
	var archive bytes.Buffer
	ext := ".tar.gz"
	if runtime.GOOS == "windows" {
		ext = ".zip"
		z := zip.NewWriter(&archive)
		w, e := z.Create("release/sing-box.exe")
		if e != nil {
			t.Fatal(e)
		}
		if _, e = w.Write(original); e != nil {
			t.Fatal(e)
		}
		if e = z.Close(); e != nil {
			t.Fatal(e)
		}
	} else {
		g := gzip.NewWriter(&archive)
		z := tar.NewWriter(g)
		if e = z.WriteHeader(&tar.Header{Name: "release/sing-box", Mode: 0700, Size: int64(len(original))}); e != nil {
			t.Fatal(e)
		}
		if _, e = z.Write(original); e != nil {
			t.Fatal(e)
		}
		if e = z.Close(); e != nil {
			t.Fatal(e)
		}
		if e = g.Close(); e != nil {
			t.Fatal(e)
		}
	}
	name := "sing-box-1.0.0-" + runtime.GOOS + "-" + runtime.GOARCH + ext
	sum := sha256.Sum256(archive.Bytes())
	badChecksum := false
	transport := http.DefaultTransport
	t.Cleanup(func() { http.DefaultTransport = transport })
	http.DefaultTransport = roundTripFunc(func(r *http.Request) (*http.Response, error) {
		var b []byte
		switch {
		case strings.Contains(r.URL.Path, "releases/latest"):
			b = []byte(fmt.Sprintf(`{"tag_name":"v1.0.0","assets":[{"name":%q,"browser_download_url":"https://example.test/archive","size":%d},{"name":"checksums.txt","browser_download_url":"https://example.test/checksums"}]}`, name, archive.Len()))
		case r.URL.Path == "/checksums":
			hash := fmt.Sprintf("%x", sum)
			if badChecksum {
				hash = strings.Repeat("0", 64)
			}
			b = []byte(hash + "  " + name)
		case r.URL.Path == "/archive":
			b = archive.Bytes()
		default:
			t.Fatalf("unexpected URL %s", r.URL)
		}
		return &http.Response{StatusCode: 200, Body: io.NopCloser(bytes.NewReader(b)), Header: make(http.Header), ContentLength: int64(len(b))}, nil
	})
	var progress DownloadProgress
	if e = k.Download(context.Background(), &progress); e != nil {
		t.Fatal(e)
	}
	installed, e := os.ReadFile(k.Binary())
	if e != nil || !bytes.Equal(original, installed) {
		t.Fatal("installed binary mismatch", e)
	}
	if progress.Downloaded.Load() != int64(archive.Len()) {
		t.Fatal("download progress mismatch")
	}
	if backup, e := os.ReadFile(k.Binary() + ".previous"); e != nil || !bytes.Equal(backup, original) {
		t.Fatal("previous kernel not preserved", e)
	}
	badChecksum = true
	if e = k.Download(context.Background(), &progress); e == nil {
		t.Fatal("invalid checksum accepted")
	}
	installed, _ = os.ReadFile(k.Binary())
	if !bytes.Equal(original, installed) {
		t.Fatal("failed download replaced working kernel")
	}
}
func TestArchiveExtraction(t *testing.T) {
	for _, ext := range []string{".zip", ".tar.gz"} {
		if _, e := extractBinary([]byte("broken"), ext); e == nil {
			t.Fatal("corrupt archive accepted")
		}
	}
}
