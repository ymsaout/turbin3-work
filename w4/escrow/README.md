## Escrow Program (w4)

This is a simple **lamports escrow** program built with Anchor.

- **`make`**: the maker locks a chosen amount of lamports in a vault PDA and creates an `escrow_state` account.
- **`take`**: a taker pays the maker the same amount of lamports and receives the locked lamports from the vault; the `escrow_state` account is closed.
- **`refund`**: the maker can cancel the escrow and recover all lamports stored in the vault; the `escrow_state` account is closed.

### Running the tests

In the `w4/escrow` directory:

```bash
yarn install        # or npm install

# Option 1: classic local validator
anchor test

# Option 2: Surfpool
# Terminal 1
surfpool start --legacy-anchor-compatibility --watch
# Terminal 2 (in w4/escrow)
yarn test:surfpool
```

### Example test logs

Here is an example of what a successful `anchor test` run looks like for this program:

```text
escrow
  ✔ make - locks lamports in the vault PDA (236ms)
  ✔ take - taker pays maker and receives the locked lamports (312ms)
  ✔ refund - maker recovers locked lamports (209ms)

  3 passing (757ms)
```

