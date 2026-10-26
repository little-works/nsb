package core

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"github.com/dop251/goja"
)

func RunHook(ctx context.Context, source, name string, input Object) (Object, error) {
	if strings.TrimSpace(source) == "" {
		return obj(input["singbox"]), nil
	}
	ctx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()
	vm := goja.New()
	vm.SetMaxCallStackSize(512)
	done := make(chan struct{})
	defer close(done)
	go func() {
		select {
		case <-ctx.Done():
			vm.Interrupt(ctx.Err())
		case <-done:
		}
	}()
	b, err := json.Marshal(input)
	if err != nil {
		return nil, err
	}
	script := strings.ReplaceAll(source, "export ", "") + "\nJSON.stringify(typeof " + name + " === 'function' ? " + name + "(" + string(b) + ") : (" + string(b) + ").singbox)"
	v, err := vm.RunString(script)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", name, err)
	}
	var result Object
	if err = json.Unmarshal([]byte(v.String()), &result); err != nil || result == nil {
		return nil, fmt.Errorf("%s must return a configuration object", name)
	}
	return result, nil
}
