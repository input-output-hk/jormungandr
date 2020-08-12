# iapyx
Test wallet based on rust chain-wallet-libs.

It contains two apis: Controller and MultiController, which can emulate wallet behavior, and also to cli utilities:

## iapyx-cli: 

command line tool to live operations on wallet (recover,voting etc)

## iapyx-load: 

load tool for generating load over backend

example:
`cargo run --bin iapyx-load -- --address 127.0.0.1:8000 --pace 100 --progress-bar-mode monitor --threads 4 -- mnemonics .\mnemonics.txt`

Where mnemonics.txt should have format like:
```
town lift follow more chronic lunch weird uniform earth census proof cave gap fancy topic year leader phrase state circle cloth reward dish survey act punch bounce
neck bulb teach illegal ry monitor claw rival amount boring provide village rival draft stone
```
