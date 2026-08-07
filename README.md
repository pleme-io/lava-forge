# lava-forge

Tatara-lisp source generator for [lava](https://github.com/pleme-io) providers.

Consumes the JSON output of `terraform providers schema -json` — the same
upstream `pangea-forge` consumes — and emits typed `(deflava-resource …)`
forms: one `.tlisp` file per resource, plus one aggregator per provider.

Where pangea-forge targets Ruby, lava-forge targets the tatara-lisp surface.

## Pipeline

```text
terraform providers schema -json          (upstream)
        │
        ▼  serde_json::from_str → ProviderSchemasFile
typed schema                              (this crate)
        │
        ▼  emit
(deflava-resource …) .tlisp forms
```

## Usage

```toml
[dependencies]
lava-forge = "0.1"
```

## Where it sits

**`lava-forge` generates the typed surface** · `lava-arch` builds an
architecture from it · `lava-stack` binds that to an environment · `magma`
executes the result. `lava-api-forge` is the sibling that skips the Terraform
provider entirely and generates from cloud API specs directly.

## License

MIT
