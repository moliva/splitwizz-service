DOCKER_COMPOSE_EXISTS = $(shell which docker > /dev/null && echo 1 || echo 0)

RUN_SERVICES = docker compose -f docker-compose.test.yml up -d > /dev/null && sleep 5

runservices:
ifeq ($(DOCKER_COMPOSE_EXISTS), 1)
	@$(RUN_SERVICES)
else
	@$(ERROR) "Install Docker in order to run the local DB"
	@exit 1;
endif

destroyservices:
	-@docker stop splitwizz_test_redis
	-@docker rm splitwizz_test_redis 
	-@docker stop splitwizz_test_db
	-@docker rm splitwizz_test_db 

tests: 
	@make runservices
	-@cargo test -- --test-threads=1
	@make destroyservices
