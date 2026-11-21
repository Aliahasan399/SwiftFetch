/**
 * SwiftFetch — popup controller.
 *
 * Every action here is an explicit user gesture. The popup reads the active
 * tab's URL only after the user opens it, and never stores page content.
 */
(function () {
  'use strict';

  const enabledInput = document.getElementById('enabled');
  const ruleSelect = document.getElementById('rule');
  const sendButton = document.getElementById('send');
  const statusEl = document.getElementById('status');
  const originLabel = document.getElementById('originLabel');

  let protocolPromise = null;
  let currentState = null;

  /**
   * The protocol comes from the classic preload bundle (`popup.html` loads
   * `shared/protocol.bundle.js` before this script) so the popup never depends
   * on a dynamic import(). The module import stays only as a fallback.
   */
  function loadProtocol() {
    if (protocolPromise === null) {
      protocolPromise = (async () => {
        if (globalThis.SwiftFetchProtocol) return globalThis.SwiftFetchProtocol;
        try {
          const module = await import(chrome.runtime.getURL('shared/protocol.js'));
          return globalThis.SwiftFetchProtocol || module;
        } catch (error) {
          return null;
        }
      })();
    }
    return protocolPromise;
  }

  function setStatus(message, kind) {
    statusEl.textContent = message || '';
    statusEl.className = kind || '';
  }

  function applyState(state) {
    currentState = state;
    enabledInput.checked = state.enabled !== false;
    if (state.origin) {
      originLabel.textContent = state.origin;
      ruleSelect.value = state.rule || 'ask';
      sendButton.disabled = false;
    } else {
      originLabel.textContent = 'No active tab.';
      ruleSelect.value = 'ask';
      sendButton.disabled = true;
    }
  }

  async function refresh() {
    try {
      const state = await chrome.runtime.sendMessage({ type: 'swiftfetch_get_state' });
      if (state && state.ok) applyState(state);
      else setStatus('SwiftFetch is unavailable.', 'error');
    } catch (error) {
      setStatus('SwiftFetch background worker is not running.', 'error');
    }
  }

  enabledInput.addEventListener('change', async () => {
    try {
      const response = await chrome.runtime.sendMessage({
        type: 'swiftfetch_set_enabled',
        enabled: enabledInput.checked,
      });
      if (response && response.ok) {
        setStatus(enabledInput.checked ? 'Capture enabled.' : 'Capture disabled.');
      } else {
        setStatus('Could not save the setting.', 'error');
      }
    } catch (error) {
      setStatus('Could not reach the background worker.', 'error');
    }
    await refresh();
  });

  ruleSelect.addEventListener('change', async () => {
    if (!currentState || !currentState.origin) return;
    try {
      await chrome.runtime.sendMessage({
        type: 'swiftfetch_set_rule',
        origin: currentState.origin,
        rule: ruleSelect.value,
      });
      const text =
        ruleSelect.value === 'allow'
          ? `${currentState.origin} will be sent without a prompt.`
          : ruleSelect.value === 'deny'
            ? `${currentState.origin} is blocked.`
            : `${currentState.origin} will ask each time.`;
      setStatus(text);
    } catch (error) {
      setStatus('Could not save the site rule.', 'error');
    }
  });

  sendButton.addEventListener('click', async () => {
    if (!currentState || !currentState.pageUrl) return;
    sendButton.disabled = true;
    setStatus('Sending…');

    const protocol = await loadProtocol();
    if (protocol && typeof protocol.validateUrl === 'function') {
      const verdict = protocol.validateUrl(currentState.pageUrl);
      if (!verdict.ok) {
        setStatus(`Blocked: ${verdict.message}`, 'error');
        sendButton.disabled = false;
        return;
      }
    }

    try {
      const response = await chrome.runtime.sendMessage({
        type: 'swiftfetch_intent',
        source: 'popup',
        payload: {
          url: currentState.pageUrl,
          suggestedName: '',
          pageOrigin: currentState.origin,
          contentType: 'application/octet-stream',
        },
      });
      if (response && response.ok) {
        setStatus(
          response.taskId ? `Sent to SwiftFetch (task ${response.taskId}).` : 'Sent to SwiftFetch.',
          'ok'
        );
      } else if (response) {
        setStatus(`${response.message || 'SwiftFetch rejected this page.'}`, 'error');
      } else {
        setStatus('No response from SwiftFetch.', 'error');
      }
    } catch (error) {
      setStatus('Could not reach SwiftFetch.', 'error');
    }
    sendButton.disabled = false;
  });

  refresh();
})();
