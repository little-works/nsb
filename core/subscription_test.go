package core

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"reflect"
	"testing"
	"time"
)

// These cases were captured from the original Rust core's complete test suite.
func TestRustSubscriptionParity(t *testing.T) {
	f, e := os.Open("testdata/rust-subscriptions.jsonl")
	if e != nil {
		t.Fatal(e)
	}
	defer f.Close()
	scanner := bufio.NewScanner(f)
	scanner.Buffer(make([]byte, 4096), 2<<20)
	count := 0
	for scanner.Scan() {
		count++
		var r struct {
			Kind     string                     `json:"kind"`
			Remote   Remote                     `json:"remote"`
			Body     string                     `json:"body"`
			Multi    bool                       `json:"multi"`
			Template string                     `json:"template"`
			Remotes  []Snapshot                 `json:"remotes"`
			Hook     *string                    `json:"hook"`
			Result   map[string]json.RawMessage `json:"result"`
		}
		if e := json.Unmarshal(scanner.Bytes(), &r); e != nil {
			t.Fatal(e)
		}
		t.Run(fmt.Sprintf("%02d_%s", count, r.Kind), func(t *testing.T) {
			var actual any
			var err error
			if r.Kind == "parse" {
				var s Snapshot
				s, err = ParseRemote(r.Remote, r.Body, r.Multi)
				s.UpdatedAt = 0
				actual = s
			} else {
				var body string
				hook := ""
				if r.Hook != nil {
					hook = *r.Hook
				}
				body, err = BuildConfig(context.Background(), r.Template, r.Remotes, hook, r.Multi)
				if err == nil {
					if e := json.Unmarshal([]byte(body), &actual); e != nil {
						t.Fatal(e)
					}
				}
			}
			if _, bad := r.Result["Err"]; bad {
				if err == nil {
					t.Fatalf("expected rejection; got %s", Pretty(actual))
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			var expected any
			if r.Kind == "parse" {
				if e := json.Unmarshal(r.Result["Ok"], &expected); e != nil {
					t.Fatal(e)
				}
				obj(expected)["updated_at"] = float64(0)
			} else {
				var body string
				if e := json.Unmarshal(r.Result["Ok"], &body); e != nil {
					t.Fatal(e)
				}
				if e := json.Unmarshal([]byte(body), &expected); e != nil {
					t.Fatal(e)
				}
			}
			b, _ := json.Marshal(actual)
			var normalized any
			_ = json.Unmarshal(b, &normalized)
			if !reflect.DeepEqual(expected, normalized) {
				t.Errorf("Rust: %s\nGo: %s", Pretty(expected), Pretty(normalized))
			}
		})
	}
	if e := scanner.Err(); e != nil {
		t.Fatal(e)
	}
	if count < 20 {
		t.Fatalf("incomplete parity fixture: %d cases", count)
	}
}
func TestHooks(t *testing.T) {
	input := Object{"singbox": Object{"outbounds": []any{}}}
	for _, name := range []string{"onGenerate", "onFinalize"} {
		t.Run(name, func(t *testing.T) {
			o, e := RunHook(context.Background(), "export function "+name+"({singbox}) {singbox.test=true;return singbox;}", name, Clone(input))
			if e != nil || o["test"] != true {
				t.Fatalf("%v %v", o, e)
			}
		})
	}
	for _, source := range []string{"function onGenerate(){throw new Error('failure')}", "function onGenerate(){return null}", "function onGenerate(){return []}", "function onGenerate(){return undefined}"} {
		if _, e := RunHook(context.Background(), source, "onGenerate", Clone(input)); e == nil {
			t.Fatal("invalid hook accepted")
		}
	}
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Millisecond)
	defer cancel()
	if _, e := RunHook(ctx, "while(true){}", "onGenerate", input); e == nil {
		t.Fatal("hook did not cancel")
	}
}
