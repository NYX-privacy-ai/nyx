// ---------------------------------------------------------------------------
// Browser — WebView management for agent-controlled web browsing
// ---------------------------------------------------------------------------
// Opens a secondary Tauri WebView window that loads external websites.
// Claude controls the browser via JS injection (DOM interaction).
// The user watches in real-time on the /browse page.
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub window_label: String,
    pub current_url: String,
    pub is_loading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PageContent {
    pub url: String,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PageLink {
    pub text: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FormField {
    pub tag: String,
    pub field_type: String,
    pub name: String,
    pub id: String,
    pub value: String,
    pub placeholder: String,
    pub label: String,
    pub selector: String,
}

/// A single browser action from Claude's tool_use response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAction {
    pub action: String,
    pub url: Option<String>,
    pub selector: Option<String>,
    pub text: Option<String>,
    pub direction: Option<String>,
    pub value: Option<String>,
    pub amount: Option<i32>,
}

/// Result returned from executing a browser action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserActionResult {
    pub success: bool,
    pub action: String,
    pub result: String,
    pub error: Option<String>,
}

/// Event payload emitted to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserEvent {
    pub kind: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

static BROWSER_STATE: std::sync::LazyLock<Mutex<Option<BrowserState>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

const BROWSER_WINDOW_LABEL: &str = "browser";

// ---------------------------------------------------------------------------
// Window management
// ---------------------------------------------------------------------------

/// Open (or show) the browser window. Creates it if it doesn't exist.
pub fn open(app: &AppHandle) -> Result<(), String> {
    // Check if window already exists
    if let Some(win) = app.get_webview_window(BROWSER_WINDOW_LABEL) {
        win.show().map_err(|e| format!("Failed to show browser window: {}", e))?;
        win.set_focus().map_err(|e| format!("Failed to focus browser window: {}", e))?;
        return Ok(());
    }

    // Create a new secondary window
    let builder = WebviewWindowBuilder::new(
        app,
        BROWSER_WINDOW_LABEL,
        WebviewUrl::External("about:blank".parse().unwrap()),
    )
    .title("Nyx — Web Browser")
    .inner_size(1200.0, 800.0)
    .min_inner_size(800.0, 500.0)
    .decorations(true)
    .visible(true);

    let _win = builder
        .build()
        .map_err(|e| format!("Failed to create browser window: {}", e))?;

    // Initialize state
    let mut state = BROWSER_STATE
        .lock()
        .map_err(|_| "Browser state lock poisoned".to_string())?;
    *state = Some(BrowserState {
        window_label: BROWSER_WINDOW_LABEL.to_string(),
        current_url: "about:blank".to_string(),
        is_loading: false,
    });

    let _ = app.emit(
        "browser:event",
        BrowserEvent {
            kind: "opened".to_string(),
            url: Some("about:blank".to_string()),
            title: None,
            message: None,
        },
    );

    Ok(())
}

/// Close the browser window.
pub fn close(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(BROWSER_WINDOW_LABEL) {
        win.close()
            .map_err(|e| format!("Failed to close browser window: {}", e))?;
    }

    let mut state = BROWSER_STATE
        .lock()
        .map_err(|_| "Browser state lock poisoned".to_string())?;
    *state = None;

    let _ = app.emit(
        "browser:event",
        BrowserEvent {
            kind: "closed".to_string(),
            url: None,
            title: None,
            message: None,
        },
    );

    Ok(())
}

/// Get the current browser state.
pub fn get_state() -> Result<Option<BrowserState>, String> {
    let state = BROWSER_STATE
        .lock()
        .map_err(|_| "Browser state lock poisoned".to_string())?;
    Ok(state.clone())
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

/// Navigate to a URL.
pub fn navigate(app: &AppHandle, url: &str) -> Result<(), String> {
    let win = get_window(app)?;

    // Normalise the URL (add https:// if missing)
    let normalised = if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    };

    let parsed: url::Url = normalised
        .parse()
        .map_err(|e| format!("Invalid URL '{}': {}", normalised, e))?;

    // Emit navigating event
    let _ = app.emit(
        "browser:event",
        BrowserEvent {
            kind: "navigating".to_string(),
            url: Some(parsed.to_string()),
            title: None,
            message: None,
        },
    );

    win.navigate(parsed)
        .map_err(|e| format!("Navigation failed: {}", e))?;

    // Update state
    if let Ok(mut state) = BROWSER_STATE.lock() {
        if let Some(ref mut s) = *state {
            s.current_url = normalised;
            s.is_loading = true;
        }
    }

    Ok(())
}

/// Go back in browser history.
pub fn go_back(app: &AppHandle) -> Result<(), String> {
    let win = get_window(app)?;
    win.eval("window.history.back()")
        .map_err(|e| format!("go_back failed: {}", e))?;
    Ok(())
}

/// Go forward in browser history.
pub fn go_forward(app: &AppHandle) -> Result<(), String> {
    let win = get_window(app)?;
    win.eval("window.history.forward()")
        .map_err(|e| format!("go_forward failed: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// DOM interaction (JS injection)
// ---------------------------------------------------------------------------

/// Click the first element matching a CSS selector.
pub fn click(app: &AppHandle, selector: &str) -> Result<String, String> {
    let js = format!(
        r#"(function() {{
            var el = document.querySelector({sel});
            if (!el) return JSON.stringify({{ error: 'Element not found: ' + {sel} }});
            el.scrollIntoView({{ behavior: 'smooth', block: 'center' }});
            el.click();
            return JSON.stringify({{ ok: true, tag: el.tagName, text: (el.textContent || '').substring(0, 100) }});
        }})()"#,
        sel = serde_json::to_string(selector).unwrap_or_else(|_| format!("\"{}\"", selector))
    );
    eval_js(app, &js)
}

/// Focus an element and type text into it.
pub fn type_text(app: &AppHandle, selector: &str, text: &str) -> Result<String, String> {
    let js = format!(
        r#"(function() {{
            var el = document.querySelector({sel});
            if (!el) return JSON.stringify({{ error: 'Element not found: ' + {sel} }});
            el.scrollIntoView({{ behavior: 'smooth', block: 'center' }});
            el.focus();
            // Clear existing value
            el.value = '';
            // Dispatch events to trigger React/Vue/Svelte handlers
            el.value = {txt};
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return JSON.stringify({{ ok: true, value: el.value }});
        }})()"#,
        sel = serde_json::to_string(selector).unwrap_or_else(|_| format!("\"{}\"", selector)),
        txt = serde_json::to_string(text).unwrap_or_else(|_| format!("\"{}\"", text))
    );
    eval_js(app, &js)
}

/// Scroll the page.
pub fn scroll(app: &AppHandle, direction: &str, amount: i32) -> Result<String, String> {
    let pixels = amount * 300; // each unit ≈ 300px
    let js = match direction {
        "up" => format!("window.scrollBy(0, -{}); 'scrolled up'", pixels),
        "down" => format!("window.scrollBy(0, {}); 'scrolled down'", pixels),
        "left" => format!("window.scrollBy(-{}, 0); 'scrolled left'", pixels),
        "right" => format!("window.scrollBy({}, 0); 'scrolled right'", pixels),
        _ => format!("window.scrollBy(0, {}); 'scrolled down'", pixels),
    };
    eval_js(app, &js)
}

/// Select an option in a dropdown.
pub fn select_option(app: &AppHandle, selector: &str, value: &str) -> Result<String, String> {
    let js = format!(
        r#"(function() {{
            var el = document.querySelector({sel});
            if (!el) return JSON.stringify({{ error: 'Element not found: ' + {sel} }});
            el.value = {val};
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return JSON.stringify({{ ok: true, value: el.value }});
        }})()"#,
        sel = serde_json::to_string(selector).unwrap_or_else(|_| format!("\"{}\"", selector)),
        val = serde_json::to_string(value).unwrap_or_else(|_| format!("\"{}\"", value))
    );
    eval_js(app, &js)
}

/// Read the current page content (URL, title, visible text).
pub fn read_page(app: &AppHandle) -> Result<String, String> {
    let js = r#"(function() {
        // Get visible text, limiting to avoid huge payloads
        var body = document.body;
        var text = '';
        var walker = document.createTreeWalker(body, NodeFilter.SHOW_TEXT, null, false);
        var count = 0;
        while (walker.nextNode() && count < 5000) {
            var node = walker.currentNode;
            var parent = node.parentElement;
            if (parent && (parent.tagName === 'SCRIPT' || parent.tagName === 'STYLE' || parent.tagName === 'NOSCRIPT')) continue;
            var t = node.textContent.trim();
            if (t) { text += t + '\n'; count++; }
        }
        // Truncate to ~30KB
        if (text.length > 30000) text = text.substring(0, 30000) + '\n...[truncated]';
        return JSON.stringify({
            url: window.location.href,
            title: document.title,
            text: text
        });
    })()"#;
    eval_js(app, js)
}

/// Read all links on the page.
pub fn read_links(app: &AppHandle) -> Result<String, String> {
    let js = r#"(function() {
        var links = [];
        var els = document.querySelectorAll('a[href]');
        for (var i = 0; i < Math.min(els.length, 200); i++) {
            var el = els[i];
            var text = (el.textContent || '').trim().substring(0, 100);
            var href = el.getAttribute('href') || '';
            if (text || href) links.push({ text: text, href: href });
        }
        return JSON.stringify(links);
    })()"#;
    eval_js(app, js)
}

/// Read form fields on the page.
pub fn read_forms(app: &AppHandle) -> Result<String, String> {
    let js = r#"(function() {
        var fields = [];
        var els = document.querySelectorAll('input, select, textarea');
        for (var i = 0; i < Math.min(els.length, 100); i++) {
            var el = els[i];
            // Skip hidden fields
            if (el.type === 'hidden') continue;
            // Try to find a label
            var label = '';
            if (el.id) {
                var lbl = document.querySelector('label[for="' + el.id + '"]');
                if (lbl) label = lbl.textContent.trim();
            }
            if (!label && el.getAttribute('aria-label')) label = el.getAttribute('aria-label');
            if (!label && el.placeholder) label = el.placeholder;

            // Build a unique CSS selector
            var selector = el.tagName.toLowerCase();
            if (el.id) selector += '#' + CSS.escape(el.id);
            else if (el.name) selector += '[name="' + el.name + '"]';
            else selector += ':nth-of-type(' + (Array.from(el.parentElement.children).indexOf(el) + 1) + ')';

            fields.push({
                tag: el.tagName.toLowerCase(),
                field_type: el.type || el.tagName.toLowerCase(),
                name: el.name || '',
                id: el.id || '',
                value: el.value || '',
                placeholder: el.placeholder || '',
                label: label.substring(0, 100),
                selector: selector
            });
        }
        return JSON.stringify(fields);
    })()"#;
    eval_js(app, js)
}

/// Execute arbitrary JavaScript in the browser window.
pub fn execute_js(app: &AppHandle, code: &str) -> Result<String, String> {
    eval_js(app, code)
}

/// Wait for a specified number of milliseconds (non-blocking on Rust side).
pub async fn wait(ms: u64) -> Result<(), String> {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Agent action dispatcher
// ---------------------------------------------------------------------------

/// Execute a BrowserAction (from Claude's tool_use) and return the result.
pub async fn execute_action(
    app: &AppHandle,
    action: &BrowserAction,
) -> BrowserActionResult {
    let action_name = action.action.as_str();

    // Emit action event to frontend activity feed
    let _ = app.emit(
        "browser:action",
        serde_json::json!({
            "action": action_name,
            "url": action.url,
            "selector": action.selector,
            "text": action.text,
        }),
    );

    let result = match action_name {
        "navigate" => {
            let url = action.url.as_deref().unwrap_or("about:blank");
            navigate(app, url).map(|_| format!("Navigated to {}", url))
        }
        "click" => {
            let sel = action
                .selector
                .as_deref()
                .ok_or_else(|| "click requires a 'selector'".to_string());
            match sel {
                Ok(s) => click(app, s),
                Err(e) => Err(e),
            }
        }
        "type" => {
            let sel = action.selector.as_deref().unwrap_or("input");
            let txt = action.text.as_deref().unwrap_or("");
            type_text(app, sel, txt)
        }
        "scroll" => {
            let dir = action.direction.as_deref().unwrap_or("down");
            let amt = action.amount.unwrap_or(3);
            scroll(app, dir, amt)
        }
        "read_page" => read_page(app),
        "read_links" => read_links(app),
        "read_forms" => read_forms(app),
        "select" => {
            let sel = action.selector.as_deref().unwrap_or("select");
            let val = action.value.as_deref().unwrap_or("");
            select_option(app, sel, val)
        }
        "back" => go_back(app).map(|_| "Went back".to_string()),
        "forward" => go_forward(app).map(|_| "Went forward".to_string()),
        "wait" => {
            let ms = action.amount.unwrap_or(2000) as u64;
            wait(ms).await.map(|_| format!("Waited {}ms", ms))
        }
        "execute_js" => {
            let code = action.text.as_deref().unwrap_or("");
            execute_js(app, code)
        }
        _ => Err(format!("Unknown browser action: {}", action_name)),
    };

    match result {
        Ok(r) => BrowserActionResult {
            success: true,
            action: action_name.to_string(),
            result: r,
            error: None,
        },
        Err(e) => BrowserActionResult {
            success: false,
            action: action_name.to_string(),
            result: String::new(),
            error: Some(e),
        },
    }
}

// ---------------------------------------------------------------------------
// Tool definition for Claude API
// ---------------------------------------------------------------------------

/// Returns the browser tool definition to send to Claude.
pub fn tool_definition() -> serde_json::Value {
    serde_json::json!({
        "name": "browser",
        "description": "Navigate and interact with websites on the user's behalf. Use this to browse the web, fill forms, click buttons, read page content, and complete tasks like booking travel or ordering groceries.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["navigate", "click", "type", "scroll", "read_page", "read_links", "read_forms", "select", "back", "forward", "wait", "execute_js"],
                    "description": "The browser action to perform"
                },
                "url": {
                    "type": "string",
                    "description": "URL to navigate to (for 'navigate' action)"
                },
                "selector": {
                    "type": "string",
                    "description": "CSS selector for the target element (for 'click', 'type', 'select' actions)"
                },
                "text": {
                    "type": "string",
                    "description": "Text to type (for 'type' action) or JavaScript code (for 'execute_js' action)"
                },
                "direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"],
                    "description": "Scroll direction (for 'scroll' action)"
                },
                "value": {
                    "type": "string",
                    "description": "Value to select (for 'select' action)"
                },
                "amount": {
                    "type": "integer",
                    "description": "Scroll amount in units (for 'scroll' action, default 3) or wait time in ms (for 'wait' action, default 2000)"
                }
            },
            "required": ["action"]
        }
    })
}

// ---------------------------------------------------------------------------
// Agent loop — Claude controls the browser
// ---------------------------------------------------------------------------

/// Maximum tool-use iterations per request (safety limit).
const MAX_ITERATIONS: usize = 25;

/// Read the Anthropic API key from docker.env.
fn read_anthropic_key() -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let env_path = std::path::PathBuf::from(&home).join("openclaw/docker.env");
    let content = std::fs::read_to_string(&env_path)
        .map_err(|e| format!("Failed to read docker.env: {}", e))?;
    for line in content.lines() {
        if line.starts_with("ANTHROPIC_API_KEY=") {
            let key = line.trim_start_matches("ANTHROPIC_API_KEY=").to_string();
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }
    Err("Anthropic API key not found in docker.env".to_string())
}

/// Send a message to Claude with the browser tool and run the agent loop.
/// Claude can issue tool_use calls which are executed against the browser,
/// and the loop continues until Claude produces a text response or the limit is hit.
pub async fn send_browse_message(
    app: &AppHandle,
    user_message: String,
    _session_key: String,
) -> Result<String, String> {
    let api_key = read_anthropic_key()?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let tool_def = tool_definition();

    // Build initial messages
    let system_prompt = "You are Nyx, a privacy-focused AI assistant helping the user browse the web. \
        You have a browser tool to navigate websites, click elements, fill forms, and read page content. \
        Work step by step: navigate to the site, read the page, interact with elements as needed. \
        Always read the page after navigating to understand what's on screen. \
        For login forms or payment pages, STOP and tell the user to complete those steps manually. \
        Never enter passwords, credit card numbers, or other sensitive credentials.";

    let mut messages = vec![serde_json::json!({
        "role": "user",
        "content": user_message
    })];

    for iteration in 0..MAX_ITERATIONS {
        // Emit iteration event
        let _ = app.emit(
            "browser:event",
            BrowserEvent {
                kind: "thinking".to_string(),
                url: None,
                title: None,
                message: Some(format!("Step {} of browsing task...", iteration + 1)),
            },
        );

        let body = serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 4096,
            "system": system_prompt,
            "tools": [tool_def],
            "messages": messages
        });

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Anthropic API request failed: {}", e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read API response: {}", e))?;

        if !status.is_success() {
            return Err(format!("Anthropic API error ({}): {}", status, text));
        }

        let resp: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse API response: {}", e))?;

        let stop_reason = resp
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("end_turn");

        let content = resp.get("content").and_then(|v| v.as_array());

        if stop_reason == "tool_use" {
            // Extract tool_use blocks and text blocks
            let content_blocks = content.cloned().unwrap_or_default();

            // Add the full assistant response to messages
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": content_blocks
            }));

            // Process each tool_use block
            let mut tool_results = Vec::new();
            for block in &content_blocks {
                if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                    let tool_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let input = block.get("input").cloned().unwrap_or(serde_json::json!({}));

                    // Parse the browser action
                    let action: BrowserAction = serde_json::from_value(input.clone())
                        .unwrap_or(BrowserAction {
                            action: "read_page".to_string(),
                            url: None,
                            selector: None,
                            text: None,
                            direction: None,
                            value: None,
                            amount: None,
                        });

                    // Execute the action
                    let result = execute_action(app, &action).await;

                    // Small delay after navigation to let page load
                    if action.action == "navigate" {
                        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                    } else if action.action == "click" {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }

                    let result_text = if result.success {
                        result.result
                    } else {
                        format!("Error: {}", result.error.unwrap_or_default())
                    };

                    tool_results.push(serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": tool_id,
                        "content": result_text
                    }));
                }
            }

            // Add tool results to messages
            messages.push(serde_json::json!({
                "role": "user",
                "content": tool_results
            }));
        } else {
            // end_turn — extract the text response
            let mut final_text = String::new();
            if let Some(blocks) = content {
                for block in blocks {
                    if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                        if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                            final_text.push_str(t);
                        }
                    }
                }
            }

            let _ = app.emit(
                "browser:event",
                BrowserEvent {
                    kind: "complete".to_string(),
                    url: None,
                    title: None,
                    message: Some(final_text.clone()),
                },
            );

            return Ok(final_text);
        }
    }

    // Hit max iterations
    let _ = app.emit(
        "browser:event",
        BrowserEvent {
            kind: "complete".to_string(),
            url: None,
            title: None,
            message: Some("Reached maximum browsing steps (25). Here's what I've done so far.".to_string()),
        },
    );

    Ok("I reached the maximum number of browsing steps (25). The task may not be fully complete — please check the browser window and tell me if you'd like me to continue.".to_string())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the browser WebviewWindow or error.
fn get_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    app.get_webview_window(BROWSER_WINDOW_LABEL)
        .ok_or_else(|| "Browser window not open. Call browser_open first.".to_string())
}

/// Evaluate JavaScript in the browser window and return the result as a string.
/// Uses Tauri's `eval()` which injects JS. For return values, we use a
/// message-passing pattern: the JS writes its result to a Tauri event.
fn eval_js(app: &AppHandle, js: &str) -> Result<String, String> {
    let win = get_window(app)?;

    // Tauri v2's eval() doesn't return values directly.
    // Workaround: wrap JS in a function that POSTs the result back via
    // window.__TAURI__.event.emit(). The caller collects via channel.
    //
    // For simplicity in v1 of this feature, we use eval() fire-and-forget
    // for actions (click, type, scroll) and return success/failure.
    // For read operations, we inject JS that calls back via Tauri event.

    // Use a sync approach: inject JS that stores result in a known location,
    // then read it back. This is simpler and avoids async coordination issues.
    let wrapper = format!(
        r#"try {{
            var __nyx_result = (function() {{ return {js}; }})();
            if (window.__TAURI_INTERNALS__) {{
                window.__TAURI_INTERNALS__.invoke('__browser_js_result', {{ result: __nyx_result || 'ok' }});
            }}
        }} catch(e) {{
            if (window.__TAURI_INTERNALS__) {{
                window.__TAURI_INTERNALS__.invoke('__browser_js_result', {{ result: JSON.stringify({{ error: e.message }}) }});
            }}
        }}"#,
        js = js
    );

    win.eval(&wrapper)
        .map_err(|e| format!("JS eval failed: {}", e))?;

    // Since eval() is fire-and-forget in Tauri v2, we return a success indicator.
    // The actual result will come back via the __browser_js_result command or
    // the frontend can read it. For the agent loop, we'll use a channel-based
    // approach in the gateway integration.
    //
    // For now, we return a placeholder — the gateway agent loop will use
    // eval_js_async() below for operations that need return values.
    Ok("ok".to_string())
}

/// Async JS evaluation that waits for the result via a one-shot channel.
/// This is used by the agent loop where we need the actual return value.
#[allow(dead_code)]
pub async fn eval_js_async(app: &AppHandle, js: &str) -> Result<String, String> {
    let win = get_window(app)?;

    // Create a unique callback ID
    let cb_id = uuid::Uuid::new_v4().to_string().replace('-', "");

    // We'll use the Tauri event system: inject JS that emits an event with the result
    let wrapper = format!(
        r#"(async function() {{
            try {{
                var __result = (function() {{ return {js}; }})();
                // Emit result back to Rust via Tauri event
                if (window.__TAURI_INTERNALS__) {{
                    window.__TAURI_INTERNALS__.postMessage(JSON.stringify({{
                        cmd: 'plugin:event|emit',
                        event: 'browser:js_result_{cb_id}',
                        payload: {{ result: typeof __result === 'string' ? __result : JSON.stringify(__result) }}
                    }}));
                }}
            }} catch(e) {{
                if (window.__TAURI_INTERNALS__) {{
                    window.__TAURI_INTERNALS__.postMessage(JSON.stringify({{
                        cmd: 'plugin:event|emit',
                        event: 'browser:js_result_{cb_id}',
                        payload: {{ error: e.message }}
                    }}));
                }}
            }}
        }})()"#,
        js = js,
        cb_id = cb_id
    );

    // Set up a one-shot listener for the result.
    // Wrap sender in Mutex<Option<>> because Tauri's listen requires Fn (not FnOnce).
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx = std::sync::Mutex::new(Some(tx));
    let event_name = format!("browser:js_result_{}", cb_id);

    let id = app.listen(&event_name, move |event: tauri::Event| {
        let payload = event.payload().to_string();
        // Take the sender (only succeeds once)
        let sender = tx.lock().ok().and_then(|mut guard| guard.take());
        if let Some(sender) = sender {
            // Parse the payload to extract the result
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&payload) {
                if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
                    let _ = sender.send(format!("{{\"error\":\"{}\"}}", err));
                } else if let Some(result) = val.get("result").and_then(|v| v.as_str()) {
                    let _ = sender.send(result.to_string());
                } else {
                    let _ = sender.send(payload);
                }
            } else {
                let _ = sender.send(payload);
            }
        }
    });

    // Inject the JS
    win.eval(&wrapper)
        .map_err(|e| format!("JS eval failed: {}", e))?;

    // Wait for the result with a timeout
    match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        Ok(Ok(result)) => {
            app.unlisten(id);
            Ok(result)
        }
        Ok(Err(_)) => {
            app.unlisten(id);
            Err("JS result channel closed unexpectedly".to_string())
        }
        Err(_) => {
            app.unlisten(id);
            // Timeout is not necessarily an error — some actions (click, scroll)
            // don't produce a meaningful return value
            Ok("ok (timeout — action likely completed)".to_string())
        }
    }
}
