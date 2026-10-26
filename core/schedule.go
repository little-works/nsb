package core

import (
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/robfig/cron/v3"
)

type schedule struct {
	spec  *cron.SpecSchedule
	years map[int]bool
}

// Rust cron uses Sunday=1 and intersects day-of-month with day-of-week.
func parseSchedule(text string) (schedule, error) {
	fields := strings.Fields(text)
	if len(fields) != 6 && len(fields) != 7 {
		return schedule{}, fmt.Errorf("cron requires six or seven fields")
	}
	days, e := scheduleValues(fields[5], 1, 7, map[string]int{"sun": 1, "mon": 2, "tue": 3, "wed": 4, "thu": 5, "fri": 6, "sat": 7})
	if e != nil {
		return schedule{}, e
	}
	yearText := "*"
	if len(fields) == 7 {
		yearText = fields[6]
	}
	years, e := scheduleValues(yearText, 1970, 2100, nil)
	if e != nil {
		return schedule{}, e
	}
	six := append([]string{}, fields[:6]...)
	six[5] = "*"
	parser := cron.NewParser(cron.Second | cron.Minute | cron.Hour | cron.Dom | cron.Month | cron.Dow)
	parsed, e := parser.Parse(strings.Join(six, " "))
	if e != nil {
		return schedule{}, e
	}
	spec := parsed.(*cron.SpecSchedule)
	spec.Dow = 0
	for d := range days {
		spec.Dow |= 1 << uint(d-1)
	}
	spec.Dom |= 1 << 63
	return schedule{spec, years}, nil
}
func scheduleValues(text string, min, max int, names map[string]int) (map[int]bool, error) {
	result := map[int]bool{}
	number := func(s string) (int, error) {
		if n, ok := names[strings.ToLower(s)]; ok {
			return n, nil
		}
		n, e := strconv.Atoi(s)
		if e != nil || n < min || n > max {
			return 0, fmt.Errorf("invalid cron value %q", s)
		}
		return n, nil
	}
	for _, entry := range strings.Split(text, ",") {
		step := 1
		part := entry
		if parts := strings.Split(entry, "/"); len(parts) == 2 {
			part = parts[0]
			var e error
			step, e = strconv.Atoi(parts[1])
			if e != nil || step <= 0 {
				return nil, fmt.Errorf("invalid cron step")
			}
		} else if len(parts) != 1 {
			return nil, fmt.Errorf("invalid cron field")
		}
		lo, hi := min, max
		if part != "*" && part != "?" {
			bounds := strings.Split(part, "-")
			var e error
			lo, e = number(bounds[0])
			if e != nil {
				return nil, e
			}
			hi = lo
			if len(bounds) == 2 {
				hi, e = number(bounds[1])
				if e != nil {
					return nil, e
				}
			} else if len(bounds) > 2 {
				return nil, fmt.Errorf("invalid cron range")
			} else if strings.Contains(entry, "/") {
				hi = max
			}
			if hi < lo {
				return nil, fmt.Errorf("invalid cron range")
			}
		}
		for v := lo; v <= hi; v += step {
			result[v] = true
		}
	}
	return result, nil
}
func (s schedule) Next(now time.Time) time.Time {
	for now.Year() <= 2100 {
		if !s.years[now.Year()] {
			year := now.Year() + 1
			for year <= 2100 && !s.years[year] {
				year++
			}
			if year > 2100 {
				return time.Time{}
			}
			now = time.Date(year, 1, 1, 0, 0, 0, 0, now.Location()).Add(-time.Second)
		}
		next := s.spec.Next(now)
		if next.IsZero() {
			// SpecSchedule searches five years; Rust permits rarer calendar matches.
			now = time.Date(now.Year()+4, 1, 1, 0, 0, 0, 0, now.Location())
			continue
		}
		if s.years[next.Year()] {
			return next
		}
		now = next
	}
	return time.Time{}
}
