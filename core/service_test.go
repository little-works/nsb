package core

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func TestGenerateDownloadsAndCaches(t *testing.T) {
	calls := 0
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		calls++
		if r.Header.Get("X-Test") != "secret" {
			t.Error("custom header missing")
		}
		_, _ = w.Write([]byte(`{"outbounds":[{"type":"vless","tag":"node"}]}`))
	}))
	defer srv.Close()
	s := Service{Store: testStore(t)}
	c := exampleConfig()
	p := c.Profiles[0]
	p.Remotes = []Remote{{Name: "source", URL: srv.URL, Headers: []Header{{"X-Test", "secret"}}, Format: "singbox", Keep: DefaultKeep()}}
	for _, refresh := range []bool{true, false, true} {
		body, _, e := s.Generate(context.Background(), c, p, refresh)
		if e != nil || body == "" {
			t.Fatal(e)
		}
	}
	if calls != 2 {
		t.Fatal(calls)
	}
}
func TestDownloadCancellationAndErrors(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { <-r.Context().Done() }))
	defer srv.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Millisecond)
	defer cancel()
	if _, e := (Service{}).Download(ctx, Remote{Name: "test", URL: srv.URL}); e == nil {
		t.Fatal("canceled request accepted")
	}
	for _, u := range []string{"file:///secret", "bad", "ftp://example.test"} {
		if _, e := (Service{}).Download(context.Background(), Remote{URL: u}); e == nil {
			t.Fatal("invalid URL accepted")
		}
	}
}
