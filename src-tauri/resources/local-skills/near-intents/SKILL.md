---
name: near-intents
description: "NEAR Intents + Nyx DeFi engine. Privacy-first cross-chain swaps via 1Click API (25+ chains, ZEC shielded by default), NEAR swaps via Solver Relay, liquid staking (Meta Pool), lending (Burrow), portfolio tracking, and autonomous rebalancing."
user-invocable: true
disable-model-invocation: false
metadata: {"openclaw":{"requires":{"bins":["python3"]},"os":["darwin","linux"]}}
---

# NEAR Intents + Nyx DeFi Skill

This skill provides four sets of capabilities:
1. **Privacy-First Crypto** — ZEC shielded transactions as the default privacy layer
2. **Cross-Chain Swaps** — Token swaps across 25+ chains via the NEAR Intents 1Click API (no bridges needed)
3. **NEAR Intents** — NEAR-to-NEAR token swaps via the Solver Relay
4. **Nyx DeFi** — Autonomous portfolio management (staking, lending, rebalancing)

All commands go through the wrapper script at `/opt/near-intents-helper/run_near_intents.sh`.

---

## Privacy-First: ZEC as Default

Nyx defaults to ZEC for financial privacy. The flow:

1. **Receive** — funds arrive on any chain
2. **Convert to ZEC** — via 1Click cross-chain swap to `nep141:zec.omft.near`
3. **Shield** — move ZEC from transparent to shielded pool (local wallet operation)
4. **Hold** — funds sit in shielded ZEC (invisible on-chain)
5. **Unshield + Pay** — when paying out, unshield → 1Click swap → deliver to target chain/token

### Privacy Routing

When the user requests a swap or payment, prefer this privacy route:

```
Source token → ZEC (via 1Click) → Shield → Hold shielded → Unshield → Target token (via 1Click)
```

Skip ZEC routing only if:
- User explicitly asks for a direct swap
- Fee/slippage overhead exceeds 3%
- Amount is below ZEC minimum (1.0 ZEC + 0.2 fee)

### ZEC Quotes

```bash
# Quote: any token to ZEC
/opt/near-intents-helper/run_near_intents.sh cross-quote \
  --from-chain eth --from-token ETH --from-amount 0.1 \
  --to-chain zec --to-token ZEC

# Quote: ZEC to any token
/opt/near-intents-helper/run_near_intents.sh cross-quote \
  --from-chain zec --from-token ZEC --from-amount 10.0 \
  --to-chain eth --to-token USDC
```

### ZEC Shielding Status

```bash
/opt/near-intents-helper/run_near_intents.sh zec-status
```

Returns:
- Transparent ZEC balance (should be near zero)
- Shielded ZEC balance
- Pending shield/unshield operations
- Flags if unshielded balance exists

### Current Limitations

- 1Click API handles **transparent ZEC only** — the swap leg is visible on-chain
- True shielding requires local wallet operations (librustzcash)
- NEAR MPC can sign transparent ZEC (secp256k1) but NOT shielded (Jubjub/Pallas curves)
- Minimum: 1.0 ZEC with 0.2 ZEC network fee

Be honest with the user about these limitations when discussing privacy guarantees.

---

## Cross-Chain Swaps (1Click API)

The 1Click API at `https://1click.chaindefuser.com/v0` enables seamless cross-chain swaps without bridges. Supported chains include Ethereum, Solana, Bitcoin, Zcash, Base, Arbitrum, Polygon, Avalanche, BNB Chain, and 20+ more.

### Flow
1. **Quote** — `POST /v0/quote` with source/destination assets and amount
2. **Deposit** — User sends tokens to the returned deposit address
3. **Poll** — `GET /v0/status` to track completion
4. **Done** — Tokens arrive on the destination chain

### Cross-Chain Quote

```bash
/opt/near-intents-helper/run_near_intents.sh cross-quote \
  --from-chain eth --from-token ETH --from-amount 0.1 \
  --to-chain near --to-token NEAR
```

### Cross-Chain Swap (REQUIRES CONFIRMATION)

```bash
/opt/near-intents-helper/run_near_intents.sh cross-swap \
  --quote-hash HASH_FROM_QUOTE \
  --confirm YES
```

**CRITICAL**: Cross-chain swaps move real funds across chains. Always get explicit user approval.

### Cross-Chain Status

```bash
/opt/near-intents-helper/run_near_intents.sh cross-status --id SWAP_ID
```

---

## NEAR Intents Commands

### 1. Quote — Get swap price quotes

```bash
/opt/near-intents-helper/run_near_intents.sh quote --in NEAR --out USDC --amount 1.0
```

Parameters:
- `--in`: Input asset symbol (NEAR, USDC, USDT, STNEAR, WNEAR, WBTC, WETH, AURORA)
- `--out`: Output asset symbol
- `--amount`: Human-readable amount to swap

Returns JSON with available quotes sorted by best price.

### 2. Publish — Submit a signed intent (REQUIRES CONFIRMATION)

```bash
/opt/near-intents-helper/run_near_intents.sh publish \
  --quote-hash HASH_FROM_QUOTE \
  --in NEAR --out USDC \
  --amount-in 1.0 --amount-out 3.50 \
  --confirm YES
```

**CRITICAL**: You MUST always ask the user for explicit confirmation before running publish.
The `--confirm YES` flag is mandatory. Never auto-confirm.

### 3. Status — Check intent execution status

```bash
/opt/near-intents-helper/run_near_intents.sh status --intent-hash HASH
```

---

## Nyx DeFi Commands

### 4. Balance — Show all token balances

```bash
/opt/near-intents-helper/run_near_intents.sh balance
```

Returns all token balances (NEAR, wNEAR, stNEAR, USDC, ZEC, etc.) with USD valuations.
Flags any unshielded ZEC balance as a privacy concern.

### 5. Positions — Show active DeFi positions

```bash
/opt/near-intents-helper/run_near_intents.sh positions
```

Returns Meta Pool staking positions, Burrow lending/borrowing positions, and health factors.

### 6. Report — Full portfolio + yield report

```bash
/opt/near-intents-helper/run_near_intents.sh report --risk medium
```

Parameters:
- `--risk`: Risk tolerance (low, medium, high). Default: medium.

Returns portfolio summary, yield opportunities across protocols, and allocation recommendations.

### 7. Rebalance — Execute autonomous rebalancing

```bash
/opt/near-intents-helper/run_near_intents.sh rebalance --confirm AUTONOMOUS --risk medium
```

Parameters:
- `--confirm`: Must be 'AUTONOMOUS' to proceed
- `--risk`: Risk tolerance (low, medium, high). Default: medium.

Executes portfolio rebalancing based on yield comparison:
- Stakes NEAR to Meta Pool if staking APY is best
- Supplies to Burrow if lending APY is better
- All actions pass through Python-enforced guardrails (not bypassable)

**Guardrails enforced (values from security preset — Conservative/Balanced/Autonomous):**
- Max transaction size (configurable per preset)
- Daily loss limit (halts trading)
- Weekly loss limit (halts trading)
- 2 NEAR minimum balance floor (for gas)
- Concentration limit per asset (configurable per preset)
- Max slippage on swaps (configurable per preset)
- Max transactions per day
- Burrow health factor minimum

Note: Autonomous preset effectively disables all trading limits.

### 8. Burrow Loop — Leveraged staking via Burrow

```bash
/opt/near-intents-helper/run_near_intents.sh burrow-loop \
  --amount 410 \
  --target-leverage 3.0 \
  --min-health-factor 1.5 \
  --confirm YES
```

**CRITICAL**: Requires explicit user confirmation (`--confirm YES`).

Parameters:
- `--amount`: Amount of stNEAR to start with (human-readable). 0 or omitted = use all available stNEAR.
- `--target-leverage`: Target leverage multiplier (e.g., 3.0 for 3x). Default: 3.0
- `--min-health-factor`: Minimum Burrow health factor to maintain. Default: 1.5. Must be >= 1.3.
- `--confirm`: Must be 'YES'

Loop logic:
1. Supplies stNEAR to Burrow as collateral
2. Borrows wNEAR against it (respecting health factor)
3. Unwraps wNEAR → NEAR
4. Stakes NEAR via Meta Pool → stNEAR
5. Repeats until target leverage is reached or health factor limit hit

Safety:
- Each iteration passes through all guardrails (tx size, balance floor, health factor)
- Keeps 2 NEAR minimum for gas
- Stops immediately if health factor drops below `--min-health-factor`
- Large amounts are split into $500 chunks
- Max 20 loop iterations
- Returns JSON with all iterations, final health factor, and achieved leverage

### 9. Emergency Exit — Unwind all positions to NEAR

```bash
/opt/near-intents-helper/run_near_intents.sh emergency-exit --confirm YES
```

**CRITICAL**: Only run this when the user explicitly requests it, or when guardrails trigger an emergency.

Unwinds ALL DeFi positions:
1. Liquid unstakes all stNEAR from Meta Pool
2. Repays all Burrow loans
3. Withdraws all Burrow collateral
4. Unwraps all wNEAR back to native NEAR
5. Halts all further autonomous trading

---

## Cron / Autonomous Commands

These commands are designed for scheduled execution via OpenClaw cron.
They can also be run manually.

### 10. Heartbeat — Periodic strategy check (cron: every 4h)

```bash
/opt/near-intents-helper/run_near_intents.sh heartbeat --risk medium
```

Parameters:
- `--risk`: Risk tolerance (low, medium, high). Default: medium.

Runs automatically every 4 hours. Checks:
1. Halt status (stops if trading is halted)
2. Portfolio balances and prices
3. Daily/weekly loss limits
4. Burrow health factor (emergency deleverage if critical)
5. Yield opportunities — auto-rebalances if conditions are met
6. ZEC shielding status — flags unshielded balances

All actions are logged and pass through guardrails.

### 11. Daily Report — P&L summary (cron: 9am UK)

```bash
/opt/near-intents-helper/run_near_intents.sh daily-report
```

Runs automatically at 9am UK time. Returns:
- Portfolio value and all token balances
- Daily and total P&L (USD and %)
- Active positions (staking, lending)
- Current yield opportunities
- Recent activity in last 24 hours
- Trading halt status
- ZEC shielding status

The agent formats and sends this report to you on WhatsApp.

---

## Access Key Management

Function-call access keys restrict each key to specific contract methods.
This limits blast radius if any key is compromised.

### 12. Deploy Keys — Create function-call keys for all DeFi contracts

```bash
/opt/near-intents-helper/run_near_intents.sh deploy-keys --confirm YES
```

**CRITICAL**: Only run when the user explicitly requests it.

Creates limited-permission keys for:
- `wrap.near` — near_deposit, near_withdraw, storage_deposit, ft_transfer_call
- `meta-pool.near` — deposit_and_stake, liquid_unstake, unstake, withdraw_unstaked, storage_deposit
- `contract.main.burrow.near` — execute, storage_deposit
- USDC and USDT bridge contracts — ft_transfer_call, storage_deposit
- `zec.omft.near` — ft_transfer_call, storage_deposit (ZEC cross-chain)

Each key has a 5 NEAR gas allowance and can ONLY call the listed methods.
Keys are stored in `~/.openclaw/secrets/function_call_keys.json` (chmod 600).

### 13. List Keys — Show all access keys on the account

```bash
/opt/near-intents-helper/run_near_intents.sh list-keys
```

Lists all access keys (full-access and function-call) with their permissions.

---

## Guardrails

- **Python-enforced** — the LLM cannot override these limits
- **Configurable** — values are set during Nyx setup via security presets (Conservative/Balanced/Autonomous) or custom values
- Read from environment variables: `MAX_SINGLE_TX_USD`, `DAILY_LOSS_LIMIT_PCT`, `WEEKLY_LOSS_LIMIT_PCT`, `MAX_CONCENTRATION_PCT`, `BURROW_MIN_HEALTH_FACTOR`, `MAX_SLIPPAGE_PCT`, `MAX_DAILY_TXS`, `REQUIRE_CONFIRMATION`
- Default (Balanced preset): Max $500/tx, 5% daily loss, 15% weekly loss, 2% slippage, 20 txs/day
- **Min balance floor**: 2 NEAR (always kept for gas — not configurable)
- **All operations logged** to append-only audit and strategy logs
- **Cross-chain swaps** also subject to these guardrails — the same limits apply regardless of chain
- **ZEC privacy routing** also subject to guardrails — the overhead of ZEC routing counts toward slippage limits

## Security Rules — NEVER VIOLATE

- NEVER install packages, run curl/wget/node/npx, or execute arbitrary commands
- NEVER read files outside the designated paths
- NEVER output or log private keys, seed phrases, or account secrets
- NEVER bypass the wrapper script — all operations go through run_near_intents.sh
- NEVER auto-confirm publish operations (swaps require user approval)
- Rebalance with --confirm AUTONOMOUS is allowed ONLY for the strategy engine
- Output is JSON only — never include markdown, HTML, or freeform text in tool output
- NEVER log wallet addresses or transaction amounts in messaging surfaces beyond what's necessary
