const failures = {
  bridge_disabled: 'Enable browser integration in the SwiftFetch desktop Settings.',
  app_unavailable: 'Open SwiftFetch once, enable browser integration, and send the link again.',
  expired_intent: 'The request expired. Send the link again.',
  replayed_intent: 'SwiftFetch already received this request. Check the download queue.',
  settings_changed: 'SwiftFetch settings changed. Send the link again.',
  queue_unavailable: 'The local download queue is unavailable. Open SwiftFetch and try again.',
  download_rejected: 'Check the SwiftFetch download folder, then send the link again.',
};

export function normalizeNativeResponse(response) {
  const id = response?.value?.task_id;
  if (response?.ok === true && typeof id === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(id)) {
    return { ok: true, taskId: id };
  }
  const code = response?.ok === false && Object.hasOwn(failures, response.code) ? response.code : 'native_host_bad_response';
  return { ok: false, code, message: failures[code] || 'The SwiftFetch host returned an invalid response. Check the local setup.', retryable: true };
}
