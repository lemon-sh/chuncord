# chuncord
Tool that allows for uploading large files to Discord in parts using webhooks.

## AUR package

If you're using Arch Linux, there's an [AUR package](https://aur.archlinux.org/packages/chuncord) available.

## Getting started
Note: You need to have the Rust toolchain installed.

- Install Chuncord:
  ```sh
  cargo install --git https://git.lemonsh.moe/lemon/chuncord --tag 0.1
  ```

- Add a Discord webhook
  ```sh
  chuncord webhook add mywebhook <webhook URL>
  ```

- Upload something
  ```sh
  chuncord upload ~/Downloads/frog.tar
  ```

- Download it
  ```sh
  chuncord download <index URL from the upload step>
  ```

- Delete it
  ```sh
  chuncord delete <MID from the upload step>
  ```

- For more help, see `./chuncord --help` and the [ArchWiki article](https://wiki.archlinux.org/title/Chuncord).
