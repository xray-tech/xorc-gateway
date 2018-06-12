# XORC Gateway

The main gateway to the XORC platform. The main routes are:

- `OPTIONS` to `/`: For Javascript clients to get the CORS headers.
- `POST` to `/`: To send events for the XORC OAM.
- `GET` to `/metrics`: If the endpoint answers, the service works. Prints
  metrics in Prometheus' format.
  
## Dependencies

XORC Gateway is written with Rust and should always be possible to compile
with the latest stable version. The de-facto way of getting the latest Rust is
with [rustup](https://rustup.rs/):

```bash
> curl https://sh.rustup.rs -sSf | sh
> rustup update
> rustup default stable
```

To check that everything works:

```bash
> rustc --version
rustc 1.26.0 (a77568041 2018-05-07)
> cargo --version
cargo 1.26.0 (0e7c5a931 2018-04-06)
```

Some of the crates used in the project have dependencies to certain system
libraries and tools, for Ubuntu 18.04 you get them with:

```bash
> sudo apt install build-essential libssl-dev automake ca-certificates libffi-dev protobuf-compiler
```

## Testing

The project uses [Protocol
Buffers](https://developers.google.com/protocol-buffers/) for event schemas.
Building the project should generate the corresponding Rust structs to be used
in the code. By default the protobuf classes are included as a submodule, which
must be imported to the project tree:

```bash
> git submodule update --init
```

To get correct country codes to the events, the system uses Maxmind's country
code database and for the development it is possible to use the free lite
version. By default the system expects to find the file from
`resources/GeoLite2-Country.mmdb`, but can be changed with the `GEOIP`
environment variable. Due to licensing issues we can't provide the file with the
repository, but it's freely available from [Maxmind's
website](https://geolite.maxmind.com/download/geoip/database/GeoLite2-Country.tar.gz).

Now it is possible to test the project without errors or warnings:

```bash
> cargo test
   Compiling xorc-gateway v0.1.0 (file:///home/pimeys/code/xorc-gateway)
    Finished dev [unoptimized + debuginfo] target(s) in 8.15 secs
     Running target/debug/deps/xorc_gateway-32f227ea61bfcfef

running XX tests

...
...
...

test result: ok. XX passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Development setup

To run XORC gateway, the required services can be created and started with
`docker-compose`.

To build everything, only needed for the first time or when changing something
in the configuration:

```bash
> docker-compose build
```

To run the services:

```bash
> docker-compose up
```

Configuration to use these local services is in `config/config.toml.developemt`,
so to start XORC gateway with it, including logging and stacktraces:

```bash
> env RUST_STACKTRACE=1 RUST_LOG=info CONFIG=config/config.toml.development cargo run
```

## Configuration

The system is configuration is handled through a
[toml](https://github.com/toml-lang/toml) file and a few environment variables.

### Environment variables

variable    | description                                                   | example
------------|---------------------------------------------------------------|----------------------------------
`PORT`      | The port to listen                                            | `1337`
`CONFIG`    | The configuration file location                               | `/etc/xorc-gateway/config.toml`
`SECRET`    | The server secret for encrypting and decrypting the device id | `<<URL_SAFE_BASE64_DATA_NO_PAD>>`
`GEOIP`     | The maxmind GeoIp2 Country database mmdb location             | `./resources/GeoLite2-Country.mmdb`
`RUST_LOG`  | Log level, either `debug`, `info`, `warn` or `error`          | `info`
`RUST_GELF` | If set, logs to Graylog                                       | `graylog.service.consul:12201`
`RUST_ENV`  | `staging` or `production`                                     | `staging`

### Required options

section       | key                     | description                                                 | example
--------------|-------------------------|-------------------------------------------------------------|------------------------
`[gateway]`   | `address`                 | The IP and port the server listens to                       | `"0.0.0.0:1337"`
`[gateway]`   | `threads`                 | Number of worker threads for the server                     | `4`
`[gateway]`   | `process_name_prefix`   | The prefix how worker threads are named in the process list | `"sdk-gateway-worker-"`
`[gateway]`   | `default_token`          | Base64 encoded token used if app does not have one set      | `"<<HEXSTRING_DATA>>"`
`[gateway]`   | `allow_empty_signature` | If true, system doesn't require a signed payload            | `false`
`[kafka]`     | `brokers`                 | A list of Kafka brokers separated with a comma              | `"kafka:9092,kafka:9093"`
`[kafka]`     | `topic`                   | The topic we should write the incoming events               | `"test.foobar"`
`[rabbitmq]`  | `exchange`                | The exchange we should write the incoming events            | `"test-foobar"`
`[rabbitmq]`  | `vhost`                   | Virtual host, by if none, should be `/`                     | `"/"`
`[rabbitmq]`  | `host`                    | Hostname                                                    | `"localhost"`
`[rabbitmq]`  | `port`                    | Port                                                        | `5672`
`[rabbitmq]`  | `login`                   | Username                                                    | `"guest"`
`[rabbitmq]`  | `password`                | Password                                                    | `"guest"`
`[aerospike]` | `nodes`                   | A list of Aerospike nodes to connect                        | `"as:3001,as:3002"`
`[aerospike]` | `namespace`               | The namespace/environment                                   | `"staging"`

### Optional options

#### Cross-Origin Resource Sharing

If enabled, allows JavaScript clients to use the API. 

section| key                     | description                                                 | example
-------|-------------------------|-------------------------------------------------------------|------------------------
`[cors]` | `allowed_methods`        | The contents of the `Access-Control-Allowed-Methods` header | `"GET,POST"`
`[cors]` | `allowed_headers`        | The contents of the `Access-Control-Allowed-Headers` header | `"Content-Type,Content-Length"`

If including the `[cors]` section in the configuration, the config must have at
least one `[[origins]]` included.

section     | key     | description                 | example
------------|---------|-----------------------------|-----------------------------------------------
`[[origins]]` | `app_id` | The application ID          | `420`
`[[origins]]` | `allowed` | An array of allowed origins | `["https://reddit.com", "https://google.com"]`

#### PostgreSQL

If enabled, the system will periodically fetch Application token and secrets
from the CRM PostgreSQL database.

section    | key           | description                                              | example
-----------|---------------|----------------------------------------------------------|-----------------------------------------------
`[postgres]` | `uri`           | The URI to the server                                    | `"postgres://login:password@host:port/database"`
`[postgres]` | `pool_size`    | The maximum number of open connections                   | `1`
`[postgres]` | `min_idle`     | The minimum number of idle connections                   | `1`
`[postgres]` | `idle_timeout` | If idle, how many milliseconds to keep a connection open | `90000`
`[postgres]` | `max_lifetime` | The maximum amount of time to keep a connection open     | `1800000`

If the section doesn't exist, the config must have at least one `[[test_apps]]` included.

section        | key             | description                                                         | example
---------------|-----------------|---------------------------------------------------------------------|-----------------------
`[[test_apps]]` | `app_id`         | The application ID                                                  | `420`
`[[test_apps]]` | `token`           | The application token that should match the `D360-Api-Token` header | `"<<HEXSTRING_DATA>>"`
`[[test_apps]]` | `secret_android` | Requests from Android platform should be signed with this           | `"<<HEXSTRING_DATA>>"`
`[[test_apps]]` | `secret_ios`     | Requests from iOS platform should be signed with this               | `"<<HEXSTRING_DATA>>"`
`[[test_apps]]` | `secret_web`     | Requests from web platform should be signed with this               | `"<<HEXSTRING_DATA>>"`

### Code Architecture

The
[gateway.rs](https://github.com/360dialog/xorc-gateway/blob/master/src/gateway.rs)
has the main server functionality. The server is mainly built on top of
[Hyper](https://github.com/hyperium/hyper) and [Tokio](https://tokio.rs)
libraries, running on top of a thread pool where each thread holds a reactor
loop making the system asynchronous by its nature. This means all blocking code
should be inside a
[blocking](https://docs.rs/tokio-threadpool/0.1/tokio_threadpool/fn.blocking.html)
block or if possible, all IO should be managed by asynchronous libraries.

The incoming data is parsed with [Serde](https://serde.rs/), the rules can be
found from the
[events/input](https://github.com/360dialog/xorc-gateway/tree/master/src/events/input).

Data passed to backend should be sent as Protocol Buffers. Conversion functionality is in 
[events/input](https://github.com/360dialog/xorc-gateway/tree/master/src/events/input)
and the structs are automatically generated every time the project is built.

The request pipeline is:

- Parse JSON (in
  [gateway.rs](https://github.com/360dialog/xorc-gateway/blob/master/src/gateway.rs)
  and
  [events/input](https://github.com/360dialog/xorc-gateway/tree/master/src/events/input))
- Check authentication for the application (in
  [app_registry.rs](https://github.com/360dialog/xorc-gateway/tree/master/src/app_registry.rs)
  for the validation,
  [encryption.rs](https://github.com/360dialog/xorc-gateway/tree/master/src/encryption.rs) +
  [context.rs](https://github.com/360dialog/xorc-gateway/tree/master/src/context.rs)
  for device id decrytpion)
- If needed, load a device ID from Aerospike, save a new ID back to Aerospike
  ([entity_storage.rs](https://github.com/360dialog/xorc-gateway/tree/master/src/entity_storage.rs) +
  [gateway.rs](https://github.com/360dialog/xorc-gateway/blob/master/src/gateway.rs))
- Send a protobuf event to Kafka and RabbitMQ
  ([gateway.rs](https://github.com/360dialog/xorc-gateway/blob/master/src/gateway.rs) +
  [bus](https://github.com/360dialog/xorc-gateway/tree/master/src/bus))
- Respond back to the client
  ([gateway.rs](https://github.com/360dialog/xorc-gateway/blob/master/src/gateway.rs))
