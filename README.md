# hcloud-prom-filesd

hcloud-prom-filesd is a tool for generating service discovery files for Prometheus out of Hetzners Cloud service.

## Installation

Use the package manager [cargo](https://doc.rust-lang.org/cargo/index.html) to install hcloud-prom-filesd.

```bash
cargo install --git https://gitlab.com/famedly/tools/hcloud-prom-filesd.git
```

## Usage

Create a config file (see `config.sample.yaml` in the repository root).
Then run `hcloud-prom-filesd --config path/to/config.yaml`.

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[AGPL-3.0](https://choosealicense.com/licenses/agpl-3.0/)
