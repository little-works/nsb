package main

import (
	"flag"
	"github.com/egoist/mygo"
	"log"
	"nsb/gui/app"
	"nsb/gui/desktop"
)

func main() {
	dev := flag.Bool("dev", mygo.IsDev(), "use gui/dev-config and gui/dev-data")
	configDir := flag.String("config-dir", "", "configuration directory")
	dataDir := flag.String("data-dir", "", "data directory")
	flag.Parse()
	c, d, e := desktop.DefaultPaths(*dev)
	if e != nil {
		log.Fatal(e)
	}
	if *configDir != "" {
		c = *configDir
	}
	if *dataDir != "" {
		d = *dataDir
	}
	log.Print("Starting NSB native desktop")
	m, e := desktop.Open(c, d)
	if e != nil {
		mygo.App.WhenReady(func() {
			_, _ = mygo.Dialog.Message(mygo.MessageOptions{Type: mygo.MessageError, Message: "NSB could not load its configuration", Detail: e.Error(), Buttons: []string{"OK"}})
			mygo.App.Quit()
		})
		if err := mygo.App.Run(); err != nil {
			log.Print(err)
		}
		return
	}
	if e = app.New(m).Run(); e != nil {
		log.Fatal(e)
	}
}
