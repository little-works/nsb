package desktop

import (
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func TestLatencyDoesNotHoldManagerLock(t *testing.T) {
	entered := make(chan struct{})
	release := make(chan struct{})
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		close(entered)
		select {
		case <-release:
			fmt.Fprint(w, `{"delay":42}`)
		case <-r.Context().Done():
		}
	}))
	defer server.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	m := &Manager{kernel: &Kernel{controller: server.URL}}
	done := make(chan error, 1)
	go func() {
		delay, err := m.Latency(ctx, "node")
		if err == nil && delay != 42 {
			err = fmt.Errorf("unexpected delay %d", delay)
		}
		done <- err
	}()
	select {
	case <-entered:
	case <-ctx.Done():
		t.Fatal("latency request did not start")
	}
	if !m.mu.TryLock() {
		t.Fatal("latency probe blocked polling and kernel operations")
	}
	m.mu.Unlock()
	close(release)
	select {
	case err := <-done:
		if err != nil {
			t.Fatal(err)
		}
	case <-ctx.Done():
		t.Fatal("latency request did not finish")
	}
}

func TestLatencyRejectsEmptyOrZeroDelay(t *testing.T) {
	for _, body := range []string{`{}`, `{"delay":0}`, `{"delay":-1}`} {
		t.Run(body, func(t *testing.T) {
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { fmt.Fprint(w, body) }))
			defer server.Close()
			m := &Manager{kernel: &Kernel{controller: server.URL}}
			if _, err := m.Latency(context.Background(), "node"); err == nil {
				t.Fatal("invalid latency accepted")
			}
		})
	}
}
