# chuncord
Tool that allows for uploading large files to Discord in parts using webhooks.

## Getting started
Note: You need to have Git and the Rust toolchain installed.

- Clone and compile `chuncord`:
  ```sh
  git clone https://git.lemonsh.moe/lemon/chuncord
  cd chuncord
  cargo build --release
  ```
  `chuncord` will be in `target/release/chuncord`.

- Add a Discord webhook
  ```sh
  cd target/release
  ./chuncord webhook add mywebhook <webhook URL>
  ```

- Upload something
  ```sh
  ./chuncord upload ~/Downloads/frog.tar
  ```

- For more help, see `./chuncord --help`