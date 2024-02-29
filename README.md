# Alien-Cake-Addict
A demo game for Z4 engine & zkVM, game from [Bevy Example](https://github.com/bevyengine/bevy/blob/main/examples/games/alien_cake_addict.rs).

[Play Online (Comming soon)]()

## Contents
- `contracts` - Solidity contracts for matchmaking & record
- `node` - Customized Z4 node only for game
- `circuit` - Game logic circuits that can be run on universal Z4 nodes(zkVM)
- `game` - Game UI and UX

## Install
### Deploy contracts
Now, we are using z4 `Demo` contracts, so goto z4/contracts/solidity
- npx hardhat node
- npm run deploy

### Running a Z4 node
- cd `node`
- `cp .env-template .env`
- `cargo run`

### Open game in browser
- cd `game`
- [config wasm in local](https://bevy-cheatbook.github.io/platforms/wasm.html)
- `cargo run --target wasm32-unknown-unknown`

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.
