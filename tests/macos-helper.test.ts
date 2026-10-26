import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import vm from 'node:vm';

const bridgePath = 'native/macos/bridge.js';
function bridge() {
  assert.ok(existsSync(bridgePath), 'native bridge must exist');
  const posted: any[] = [];
  const timers = new Map<number, Function>();
  let nextTimer = 0;
  const lifecycle = new Map<string, Function>();
  const window: any = { addEventListener: (event: string, callback: Function) => lifecycle.set(event, callback), webkit: { messageHandlers: { orbit: { postMessage: (m: any) => posted.push(m) } } } };
  vm.runInNewContext(readFileSync(bridgePath, 'utf8'), { window, setTimeout: (fn: Function) => { timers.set(++nextTimer, fn); return nextTimer; }, clearTimeout: (id: number) => timers.delete(id), console });
  return { window, posted, timers, lifecycle };
}
test('native bridge pairs invokes and replies, propagating native errors', async () => {
  const { window, posted, timers } = bridge();
  const result = window.__ORBIT_OVERLAY__.invoke('list_sessions', { limit: 2 });
  assert.equal(posted[0].command, 'list_sessions');
  assert.equal(posted[0].args.limit, 2);
  window.__ORBIT_RECEIVE__({ type: 'reply', id: posted[0].id, ok: true, value: ['session'] });
  assert.deepEqual(await result, ['session']);
  assert.equal(timers.size, 0);
  const failure = window.__ORBIT_OVERLAY__.invoke('bad');
  window.__ORBIT_RECEIVE__({ type: 'reply', id: posted[1].id, ok: false, value: 'denied' });
  await assert.rejects(failure, /denied/);
});
test('native bridge delivers events and honors unlisten', async () => {
  const { window } = bridge();
  const events: any[] = [];
  const stop = await window.__ORBIT_OVERLAY__.listen('sessions-changed', (e: any) => events.push(e));
  window.__ORBIT_RECEIVE__({ type: 'event', event: 'sessions-changed', payload: 3 });
  assert.equal(events[0].payload, 3);
  stop();
  window.__ORBIT_RECEIVE__({ type: 'event', event: 'sessions-changed', payload: 4 });
  assert.equal(events.length, 1);
});
test('native bridge rejects abandoned requests after timeout', async () => {
  const { window, timers } = bridge();
  const result = window.__ORBIT_OVERLAY__.invoke('never');
  [...timers.values()][0]();
  await assert.rejects(result, /timed out/);
});
test('macOS helper is a separate genuine panel process with restricted web origin', () => {
  assert.ok(existsSync('native/macos/main.swift'), 'native helper must exist');
  const swift = readFileSync('native/macos/main.swift', 'utf8');
  assert.match(swift, /class OverlayPanel: NSPanel/);
  assert.match(swift, /setActivationPolicy\(\.accessory\)/);
  assert.match(swift, /\.nonactivatingPanel/);
  assert.match(swift, /\.fullScreenAuxiliary/);
  assert.match(swift, /\.canJoinAllApplications/);
  assert.match(swift, /message\.frameInfo\.isMainFrame/);
  assert.match(swift, /WKURLSchemeHandler/);
  assert.doesNotMatch(swift, /object_setClass|NSApp\.activate|loadFileURL/);
});
test('native panel reports coalesced user geometry but suppresses parent-driven geometry', () => {
  const swift = readFileSync('native/macos/main.swift', 'utf8');
  assert.match(swift, /func windowDidMove\(/);
  assert.match(swift, /func windowDidResize\(/);
  assert.match(swift, /guard !applyingParentUpdate else \{ return \}/);
  assert.match(swift, /geometryUpdate\?\.cancel\(\)/);
  assert.match(swift, /"type": "geometry"/);
  assert.match(swift, /screen\.frame\.maxY - frame\.maxY/);
});
test('native close hides reusable panels and emits one closed notification', () => {
  const swift = readFileSync('native/macos/main.swift', 'utf8');
  assert.match(swift, /func windowShouldClose\(_ sender: NSWindow\) -> Bool/);
  assert.doesNotMatch(swift, /func windowWillClose\(/);
  assert.match(swift, /panel\.orderOut\(nil\)\s+send\(\["type": "closed", "label": label\]\)\s+return false/);
});

test('directory selection has no deadline while ordinary requests still expire', async () => {
  const { window, posted, timers } = bridge();
  const selection = window.__ORBIT_OVERLAY__.invoke('overlay_choose_directory');
  assert.equal(timers.size, 0, 'human modal interactions must not have a timer');
  const ordinary = window.__ORBIT_OVERLAY__.invoke('list_sessions');
  for (const callback of [...timers.values()]) callback();
  await assert.rejects(ordinary, /timed out/);
  // All elapsed deadlines have fired; the modal still accepts its eventual result.
  window.__ORBIT_RECEIVE__({ type: 'reply', id: posted[0].id, ok: true, value: '/chosen/folder' });
  assert.equal(await selection, '/chosen/folder');
});
test('pagehide rejects all pending requests and removes timers and event listeners', async () => {
  const { window, timers, lifecycle } = bridge();
  const modal = window.__ORBIT_OVERLAY__.invoke('overlay_choose_directory');
  const ordinary = window.__ORBIT_OVERLAY__.invoke('list_sessions');
  const rejectedModal = assert.rejects(modal, /Overlay closed/);
  const rejectedOrdinary = assert.rejects(ordinary, /Overlay closed/);
  let events = 0;
  await window.__ORBIT_OVERLAY__.listen('test', () => events++);
  assert.ok(lifecycle.has('pagehide'));
  lifecycle.get('pagehide')!();
  await Promise.all([rejectedModal, rejectedOrdinary]);
  assert.equal(timers.size, 0);
  window.__ORBIT_RECEIVE__({ type: 'event', event: 'test', payload: null });
  assert.equal(events, 0);
});
test('custom scheme returns HTTP headers including a narrow CORS origin and HTML CSP', () => {
  const swift = readFileSync('native/macos/main.swift', 'utf8');
  assert.match(swift, /HTTPURLResponse\(url: url, statusCode: 200/);
  assert.match(swift, /"Access-Control-Allow-Origin": "orbit-asset:\/\/localhost"/);
  assert.match(swift, /headers\["Content-Security-Policy"\] = policy/);
});
test('native navigation or WebContent process failures report content errors without reloading', () => {
  const swift = readFileSync('native/macos/main.swift', 'utf8');
  assert.match(swift, /didFailProvisionalNavigation/);
  assert.match(swift, /didFail navigation/);
  assert.match(swift, /webViewWebContentProcessDidTerminate/);
  assert.match(swift, /NSURLErrorCancelled/);
  assert.match(swift, /"type": "content-error"/);
  assert.doesNotMatch(swift, /webView\.reload\(/);
});
