package app

import (
	"regexp"
	"strings"
)

var ansiEscape = regexp.MustCompile(`\x1b\[[0-?]*[ -/]*[@-~]`)
var logSeverity = regexp.MustCompile(`(?i)\b(TRACE|DEBUG|INFO|WARN(?:ING)?|ERROR|FATAL|PANIC)\b`)

var logLevelNames = [...]string{"TRACE", "DEBUG", "INFO", "WARN", "ERROR", "FATAL"}

type logSelection [len(logLevelNames)]bool

type logLine struct{ text, level string }

func filterLogs(body, query string, selected logSelection) []logLine {
	lines := []logLine{}
	query = strings.ToLower(strings.TrimSpace(query))
	rawLines := strings.Split(body, "\n")
	// sing-box appends records in chronological order; show the latest first.
	for i := len(rawLines) - 1; i >= 0; i-- {
		line := ansiEscape.ReplaceAllString(rawLines[i], "")
		if strings.TrimSpace(line) == "" || query != "" && !strings.Contains(strings.ToLower(line), query) {
			continue
		}
		level := strings.ToUpper(logSeverity.FindString(line))
		switch level {
		case "":
			level = "INFO"
		case "WARNING":
			level = "WARN"
		case "PANIC":
			level = "FATAL"
		}
		for j, name := range logLevelNames {
			if name == level && selected[j] {
				lines = append(lines, logLine{text: line, level: level})
				break
			}
		}
	}
	return lines
}
