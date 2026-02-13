---
name: travel
description: "Travel research, comparison, and itinerary planning. Searches flights, hotels, and transportation options across multiple providers."
user-invocable: true
disable-model-invocation: false
metadata: {"openclaw":{"requires":{"bins":[]},"os":["darwin","linux"]}}
---

# Travel Skill

Research and plan travel — flights, hotels, and transportation. Nyx acts as your travel advisor: researching options, comparing prices, and presenting recommendations.

## Philosophy

- **Research first, book later.** Always present options and get explicit approval before any booking.
- **Optimise for value, not just price.** Factor in layovers, convenience, loyalty programs, and user preferences.
- **Privacy-conscious.** Don't store passport numbers, frequent flyer numbers, or payment details in logs. Use the user's profile from `USER.md` for basic preferences only.

---

## Capabilities

### Flight Search

Use web search to find flight options across providers:

```
Search for flights:
- Route: [origin] to [destination]
- Dates: [outbound] to [return] (or one-way)
- Class: economy / business / first
- Passengers: [number]
- Preferences: direct flights preferred, specific airlines, time of day
```

Compare results from:
- Google Flights (best for price comparison)
- Skyscanner (good for flexible dates)
- Airline direct sites (for loyalty pricing)

### Hotel Search

```
Search for hotels:
- Location: [city/area]
- Dates: [check-in] to [check-out]
- Guests: [number]
- Budget: [range per night]
- Preferences: pool, gym, central location, specific chains
```

Compare results from:
- Booking.com
- Google Hotels
- Direct hotel sites

### Transportation

```
Search for transport:
- Type: car rental / train / private transfer
- Route: [pickup] to [dropoff]
- Dates: [from] to [to]
```

---

## Output Format

Always present travel options as a structured comparison:

**Flights: LHR → JFK, 15 Mar 2026**

| Option | Airline | Depart | Arrive | Stops | Duration | Price |
|--------|---------|--------|--------|-------|----------|-------|
| 1 | BA 117 | 09:00 | 12:00 | Direct | 8h | $450 |
| 2 | VS 3 | 11:30 | 14:30 | Direct | 8h | $420 |
| 3 | AA 101 | 07:15 | 13:45 | 1 (ORD) | 11h30 | $310 |

**Recommendation:** Option 2 — best balance of price and schedule.

---

## Booking Flow

When the user wants to book:

1. **Confirm details** — restate the selected option, dates, passenger names
2. **Direct to provider** — provide a deep link to the booking page on the provider's site
3. **Do NOT enter payment details** — the user completes payment themselves
4. **Track booking** — once confirmed, add to Google Calendar via gog, save confirmation details to `memory/`

### Future: API-Based Booking

When API integrations are available (Duffel for flights, Booking.com affiliate API):
- Quote and hold via API
- Present final price with all fees
- Get explicit user confirmation
- Execute booking via API
- Send confirmation to user

**Not yet implemented** — current version uses web search + provider deep links.

---

## User Preferences

Read from `USER.md` or `memory/travel-prefs.json`:

```json
{
  "home_airports": ["LHR", "LGW"],
  "preferred_airlines": ["BA", "VS"],
  "seat_preference": "aisle",
  "class_default": "economy",
  "hotel_chains_preferred": ["Marriott", "Hilton"],
  "loyalty_programs": {
    "ba_exec_club": true,
    "marriott_bonvoy": true
  },
  "dietary": null,
  "passport_country": "GB"
}
```

If no preferences file exists, ask the user on first travel request and save for future.

---

## Calendar Integration

After booking confirmation:
- Create Google Calendar events via `gog calendar create`
- Include: flight number, times, confirmation reference, hotel address
- Set reminders: 24h before departure, 2h before departure

---

## Rules

- NEVER store or log passport numbers, payment details, or frequent flyer numbers
- NEVER auto-book — always get explicit user confirmation
- NEVER present fake or made-up prices — only use real search results
- If you can't find good options, say so — don't fabricate alternatives
- Respect the user's budget constraints
- For international travel, note visa requirements if applicable
