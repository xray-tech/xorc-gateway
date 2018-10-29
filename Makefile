SHELL = /bin/sh
executable = target/release/xorc-gateway
artifactory = http://repo:katti@artifactory.service.consul:8081/artifactory/xorc_gateway
commit_id = `git log --format="%h" -n 1`
marathon = http://leader.mesos.service.consul:8080/v2/apps/
influx = "http://influxdb.service.consul:8086/write?db=deployments"
curl = `which curl`
deplicity = `which deplicity`
branch = $(shell git rev-parse --abbrev-ref HEAD)
stage = staging

ifeq ($(branch), master)
	stage = production
endif

ifeq ($(STAGE), production)
	stage = production
endif

config = deploy/$(stage).mar.template

.PHONY: help

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

update: ## Update the running Mesos configuration
	$(deplicity) -i $(influx) -m $(marathon) -j $(config) -v $(commit_id) bluegreen

auto_update: ## Update the running Mesos configuration, don't ask questions
	$(deplicity) -f -i $(influx) -m $(marathon) -j $(config) -v $(commit_id) bluegreen

upload: ## Upload the binary to the repository
	$(info $$branch is [${branch}])
	$(curl) -T $(executable) $(artifactory)/$(stage)/xorc-gateway-$(commit_id)

