package desktop

import (
	"archive/tar"
	"archive/zip"
	"bytes"
	"compress/gzip"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"runtime"
	"strings"
	"sync/atomic"
	"time"

	"nsb/core"
)

type DownloadProgress struct{ Downloaded, Total atomic.Int64 }
type Release struct {
	Tag    string `json:"tag_name"`
	Assets []struct {
		Name string `json:"name"`
		URL  string `json:"browser_download_url"`
		Size int64  `json:"size"`
	} `json:"assets"`
}

func githubDownload(ctx context.Context, url string, limit int64, progress *DownloadProgress) ([]byte, error) {
	req, e := http.NewRequestWithContext(ctx, "GET", url, nil)
	if e != nil {
		return nil, e
	}
	req.Header.Set("User-Agent", "NSB/0.1.0")
	resp, e := (&http.Client{Timeout: 10 * time.Minute}).Do(req)
	if e != nil {
		return nil, e
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("GitHub HTTP %d", resp.StatusCode)
	}
	if progress != nil {
		progress.Total.Store(resp.ContentLength)
		progress.Downloaded.Store(0)
	}
	var b bytes.Buffer
	reader := io.LimitReader(resp.Body, limit+1)
	buf := make([]byte, 64<<10)
	for {
		n, err := reader.Read(buf)
		if n > 0 {
			b.Write(buf[:n])
			if progress != nil {
				progress.Downloaded.Add(int64(n))
			}
		}
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, err
		}
	}
	if int64(b.Len()) > limit {
		return nil, fmt.Errorf("download exceeds limit")
	}
	return b.Bytes(), nil
}
func LatestRelease(ctx context.Context) (Release, error) {
	var release Release
	b, e := githubDownload(ctx, "https://api.github.com/repos/SagerNet/sing-box/releases/latest", 2<<20, nil)
	if e != nil {
		return release, e
	}
	e = json.Unmarshal(b, &release)
	return release, e
}
func (k *Kernel) Download(ctx context.Context, progress *DownloadProgress) error {
	release, e := LatestRelease(ctx)
	if e != nil {
		return e
	}
	osName := runtime.GOOS
	arch := runtime.GOARCH
	ext := ".tar.gz"
	if osName == "windows" {
		ext = ".zip"
	}
	name := "sing-box-" + strings.TrimPrefix(release.Tag, "v") + "-" + osName + "-" + arch + ext
	var archiveURL, checksumURL string
	var expectedSize int64
	for _, a := range release.Assets {
		if a.Name == name {
			archiveURL = a.URL
			expectedSize = a.Size
		}
		if strings.Contains(a.Name, "checksum") {
			checksumURL = a.URL
		}
	}
	if archiveURL == "" {
		return fmt.Errorf("no sing-box release for %s/%s", osName, arch)
	}
	if checksumURL == "" {
		return fmt.Errorf("release does not provide checksums")
	}
	checksums, e := githubDownload(ctx, checksumURL, 2<<20, nil)
	if e != nil {
		return e
	}
	expected := ""
	for _, line := range strings.Split(string(checksums), "\n") {
		parts := strings.Fields(line)
		if len(parts) >= 2 && strings.TrimPrefix(parts[1], "*") == name {
			expected = strings.ToLower(parts[0])
			break
		}
	}
	if len(expected) != 64 {
		return fmt.Errorf("checksum for %s not found", name)
	}
	archive, e := githubDownload(ctx, archiveURL, 256<<20, progress)
	if e != nil {
		return e
	}
	if expectedSize > 0 && int64(len(archive)) != expectedSize {
		return fmt.Errorf("download size mismatch")
	}
	sum := sha256.Sum256(archive)
	if hex.EncodeToString(sum[:]) != expected {
		return fmt.Errorf("sing-box checksum mismatch")
	}
	binary, e := extractBinary(archive, ext)
	if e != nil {
		return e
	}
	return k.Install(ctx, binary)
}

func (k *Kernel) Install(ctx context.Context, binary []byte) error {
	if k.Running() {
		return fmt.Errorf("stop the kernel before installing an update")
	}
	if e := os.MkdirAll(filepath.Dir(k.Binary()), 0700); e != nil {
		return e
	}
	stage, e := os.CreateTemp(filepath.Dir(k.Binary()), "sing-box-*.exe")
	if e != nil {
		return e
	}
	staged := stage.Name()
	defer os.Remove(staged)
	if _, e = stage.Write(binary); e != nil {
		stage.Close()
		return e
	}
	if e = stage.Close(); e != nil {
		return e
	}
	if e = makeExecutable(staged); e != nil {
		return e
	}
	checkCtx, cancel := context.WithTimeout(ctx, 10*time.Second)
	defer cancel()
	check := exec.CommandContext(checkCtx, staged, "version")
	hideCommand(check)
	if output, err := check.Output(); err != nil || !strings.Contains(string(output), "sing-box") {
		return fmt.Errorf("downloaded kernel failed its version check")
	}
	if old, err := os.ReadFile(k.Binary()); err == nil {
		if e = core.AtomicWrite(k.Binary()+".previous", old); e != nil {
			return e
		}
	} else if !os.IsNotExist(err) {
		return err
	}
	if e = core.AtomicWrite(k.Binary(), binary); e != nil {
		return e
	}
	return makeExecutable(k.Binary())
}
func extractBinary(data []byte, ext string) ([]byte, error) {
	read := func(r io.Reader) ([]byte, error) {
		b, e := io.ReadAll(io.LimitReader(r, 128<<20+1))
		if len(b) > 128<<20 {
			return nil, fmt.Errorf("kernel binary too large")
		}
		return b, e
	}
	if ext == ".zip" {
		z, e := zip.NewReader(bytes.NewReader(data), int64(len(data)))
		if e != nil {
			return nil, e
		}
		for _, f := range z.File {
			if path.Base(f.Name) == "sing-box.exe" && !f.FileInfo().IsDir() {
				r, e := f.Open()
				if e != nil {
					return nil, e
				}
				b, e := read(r)
				r.Close()
				return b, e
			}
		}
	} else {
		g, e := gzip.NewReader(bytes.NewReader(data))
		if e != nil {
			return nil, e
		}
		defer g.Close()
		t := tar.NewReader(g)
		for {
			h, e := t.Next()
			if e == io.EOF {
				break
			}
			if e != nil {
				return nil, e
			}
			if path.Base(h.Name) == "sing-box" && h.Typeflag == tar.TypeReg {
				return read(t)
			}
		}
	}
	return nil, fmt.Errorf("archive contains no sing-box binary")
}
