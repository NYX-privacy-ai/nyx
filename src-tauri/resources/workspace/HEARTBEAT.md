# HEARTBEAT.md

## Periodic Checks (rotate through these each heartbeat)

### Email Triage
- Quick scan of unread emails across all gog accounts
- Flag anything 🔴 URGENT to the user immediately
- Track last check timestamp in `memory/heartbeat-state.json`

### Calendar Awareness
- Check upcoming events in the next 4 hours
- Remind user of imminent meetings (< 30 min away)

### DeFi Heartbeat
- Run `/opt/near-intents-helper/run_near_intents.sh heartbeat --risk medium`
- Only alert if actions taken or errors occurred

### Portfolio Check
- If market hours and significant price movement detected, flag it
- Check ZEC shielding status — any unshielded balance should be flagged

## Rules
- Don't repeat a check if it ran < 30 minutes ago (check heartbeat-state.json)
- Late night (23:00-08:00 UK) — only check if something looks genuinely urgent
- Batch checks together to minimise API calls
- If nothing needs attention: reply HEARTBEAT_OK
