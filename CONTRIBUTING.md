# Contribution guidelines

First off, thank you for considering contributing to cloudmqtt.

If your contribution is not straightforward, please first discuss the change you
wish to make by creating a new issue before making the change.

## Reporting issues

Before reporting an issue on the
[issue tracker](https://github.com/TheNeikos/cloudmqtt/issues),
please check that it has not already been reported by searching for some related
keywords.

## Pull requests

Try to do one pull request per change.

## Developing

### Set up


#### Non-Nix setups
This is no different than other Rust projects.

```shell
git clone https://github.com/TheNeikos/cloudmqtt
cd cloudmqtt
cargo test
```

Make sure that you have at least the version as specified in the `rust-toolchain` file.

#### Nix setups

The repository is a nix flake. Be sure to have flake support enabled to check out this repository.

```shell
nix shell
cargo test
```

### Useful Commands
- Build and run the binary:

  ```shell
  cargo run --release -F bin
  ```

- Run Clippy:

  ```shell
  cargo clippy --all-targets --all-features --workspace
  ```

- Run all tests:

  ```shell
  cargo test --all-features --workspace
  ```

- Check to see if there are code formatting issues

  ```shell
  cargo fmt --all -- --check
  ```

- Format the code in the project

  ```shell
  cargo fmt --all
  ```

