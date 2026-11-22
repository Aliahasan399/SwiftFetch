/**
 * Media detection regression tests. These run against the ES module directly;
 * no real browser is required for the data-shape and URL-safety assertions.
 */
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { scanMedia, absoluteUrl } from '../shared/media-detect.js';

class FakeMedia {
  constructor(src, type, tag = 'VIDEO') {
    this.tagName = tag;
    this.src = src;
    this.currentSrc = '';
    this.type = type || '';
    this.width = 0;
    this.height = 0;
    this.srcObject = null;
    this.sourceElements = [];
  }
  querySelectorAll(selector) { return selector === 'source' ? this.sourceElements : []; }
}

describe('media detection', () => {
  it('accepts only http(s) absolute media URLs', () => {
    assert.equal(absoluteUrl('/video.mp4', 'https://example.test/page'), 'https://example.test/video.mp4');
    assert.equal(absoluteUrl('blob:https://example.test/abc', 'https://example.test/'), null);
    assert.equal(absoluteUrl('data:video/mp4;base64,AAAA', 'https://example.test/'), null);
    assert.equal(absoluteUrl('javascript:alert(1)', 'https://example.test/'), null);
  });

  it('detects accessible video and source elements without starting a download', () => {
    globalThis.HTMLMediaElement = FakeMedia;
    const video = new FakeMedia('https://example.test/video.mp4', 'video/mp4');
    const audio = new FakeMedia('https://example.test/audio.mp3', 'audio/mpeg', 'AUDIO');
    const doc = {
      baseURI: 'https://example.test/',
      querySelectorAll: (selector) => (selector === 'video, audio' ? [video, audio] : []),
    };
    const found = scanMedia(doc, 8);
    assert.equal(found.length, 2);
    assert.equal(found[0].url, 'https://example.test/video.mp4');
    assert.equal(found[0].variants.length, 1);
    assert.equal(found[0].direct, true);
    assert.equal(found[1].tag, 'audio');
  });

  it('skips media streams and blocked schemes', () => {
    globalThis.HTMLMediaElement = FakeMedia;
    const stream = new FakeMedia('blob:https://example.test/stream', '');
    stream.srcObject = {};
    const doc = {
      baseURI: 'https://example.test/',
      querySelectorAll: () => [stream],
    };
    assert.equal(scanMedia(doc, 8).length, 0);
  });
});
