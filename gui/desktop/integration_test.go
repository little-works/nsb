package desktop

import (
	"context"
	"os"
	"strings"
	"testing"
	"time"

	"nsb/core"
)

func TestRealSingBoxIntegration(t *testing.T) {
	source := os.Getenv("NSB_TEST_SINGBOX")
	if source == "" {
		t.Skip("set NSB_TEST_SINGBOX to enable local kernel integration")
	}
	b, e := os.ReadFile(source)
	if e != nil {
		t.Fatal(e)
	}
	k := NewKernel(t.TempDir())
	if e = core.AtomicWrite(k.Binary(), b); e != nil {
		t.Fatal(e)
	}
	if e = makeExecutable(k.Binary()); e != nil {
		t.Fatal(e)
	}
	t.Cleanup(func() {
		if e := k.Shutdown(); e != nil {
			t.Error(e)
		}
	})
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()
	if !strings.Contains(k.Version(ctx), "sing-box") {
		t.Fatal("invalid binary")
	}
	body, e := core.BuildConfig(ctx, `{"log":{"level":"info"},"outbounds":[{"type":"direct","tag":"direct"}]}`, nil, "", false)
	if e != nil {
		t.Fatal(e)
	}
	if e = k.Start(ctx, body, "", false); e != nil {
		t.Fatal(e)
	}
	var proxies, connections core.Object
	if e = k.Request(ctx, "GET", "/proxies", nil, &proxies); e != nil {
		t.Fatal(e)
	}
	if e = k.Request(ctx, "GET", "/connections", nil, &connections); e != nil {
		t.Fatal(e)
	}
	if proxies["proxies"] == nil || connections["connections"] == nil {
		t.Fatalf("unexpected controller responses: %v %v", proxies, connections)
	}
	if e = k.Request(ctx, "PUT", "/proxies/PROXY", core.Object{"name": "direct"}, nil); e != nil {
		t.Fatal(e)
	}
	if e = k.Request(ctx, "PATCH", "/configs", core.Object{"mode": "global"}, nil); e != nil {
		t.Fatal(e)
	}
	if !strings.Contains(k.LogTail(), "sing-box started") {
		t.Fatalf("missing startup log: %s", k.LogTail())
	}
	if e = k.Stop(); e != nil {
		t.Fatal(e)
	}
	if k.Running() {
		t.Fatal("real kernel was not stopped")
	}
}
