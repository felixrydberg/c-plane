package main

import (
	"os"

	"github.com/cplane/cplane/registry/driver"
	cplanemiddleware "github.com/cplane/cplane/registry/middleware"
	"github.com/distribution/distribution/v3/registry"
	_ "github.com/distribution/distribution/v3/registry/storage/driver/s3-aws"
)

func main() {
	driver.Register()
	registry.RegisterHandler(cplanemiddleware.New)
	registry.RootCmd.RemoveCommand(registry.GCCmd)
	if err := registry.RootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
