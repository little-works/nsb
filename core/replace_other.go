//go:build !windows

package core

import "os"

func replaceFile(from, to string) error { return os.Rename(from, to) }
