# Alien Cake Addict front-end

game based on Bevy engine.

### build
1. install rustup & latest stable rust

2. `rustup target add wasm32-unknown-unknown`

3. `cargo install wasm-server-runner` for dev, [more detail](https://bevy-cheatbook.github.io/platforms/wasm.html)
```
[target.wasm32-unknown-unknown]
runner = "wasm-server-runner"
```
add to `.cargo/config.toml`

4. `cargo run --target wasm32-unknown-unknown` for dev

5. `cargo build --release --target wasm32-unknown-unknown` for prod, [more detail](https://bevy-cheatbook.github.io/platforms/wasm/webpage.html)

6. `wasm-bindgen --no-typescript --out-name bevy_game --out-dir wasm --target web ../target/wasm32-unknown-unknown/release/alien-cake-addict.wasm`

## License

This project is licensed under [GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html).
