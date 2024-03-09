
# Splitwizz service

## Development setup

Make sure to have a local Postgres DB called `splitwizz`, run the migration scripts:

```bash
cargo install sqlx-cli
sqlx migrate run
```

And run the development environment with the following command:

```bash
cargo watch -x run
```

## Production build

Before making the image build, since `sqlx` will need to run offline during the build phase in the Docker environment, make sure to build the offline data file with:

```bash
cargo sqlx prepare
```

After that just go ahead and build the production image, make sure to bump both versions:

```bash
./build-docker.sh
```

And the migrations image as well:


```bash
./build-docker-migrations.sh
```

After checking that everything is good you can run the same commands now with the publish flag on `-p`
