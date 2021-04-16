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

### Filters

Usually, your hosts are not 100% identical and need slightly different
monitoring per host/set of hosts. To accomodate this, you can use
`hcloud-prom-filesd` to filter based on labels. In the config file, set up
`filters` like this:

```yaml
filters:
- - test1
  - test2
```
to create a tree like this:
```text
out
├── all.json
├── test1-is-empty
│   ├── test2-is-empty.json
│   ├── test2-is-set.json
│   ├── test2-is-not-set.json
│   ├── test2-is-example1.json
│   ├── test2-is-not-example1.json
│   ├── test2-is-example2.json
│   └── test2-is-not-example2.json
├── test1-is-set
│   ├── ...
│   └── ...
├── test1-is-not-set
│   ├── ...
│   └── ...
├── test1-is-example1
│   ├── ...
│   └── ...
├── test1-is-not-example1
│   ├── ...
│   └── ...
├── test1-is-example2
│   ├── ...
│   └── ...
└── test1-is-not-example2
    ├── ...
    └── ...
```

Assuming you have a host `host1` with labels `{"test1":"example1", "test2":
""}`, the host would show up in the following files:

- `test1-is-set/test2-is-empty.json`
- `test1-is-set/test2-is-set.json`
- `test1-is-example1/test2-is-empty.json`
- `test1-is-example1/test2-is-set.json`
- `test1-is-not-example2/test2-is-empty.json`
- `test1-is-not-example2/test2-is-set.json`

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[AGPL-3.0](https://choosealicense.com/licenses/agpl-3.0/)
