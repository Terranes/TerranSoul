/**
 * Tests for the emotion tag parser.
 */
import { describe, it, expect } from 'vitest';
import { parseTags, stripTags } from './emotion-parser';

describe('emotion-parser — parseTags', () => {
  it('parses [happy] tag', () => {
    const result = parseTags('[happy] Great to see you!');
    expect(result.emotion).toBe('happy');
    expect(result.text).toBe('Great to see you!');
  });

  it('parses [sad] tag', () => {
    const result = parseTags('[sad] I am sorry to hear that.');
    expect(result.emotion).toBe('sad');
    expect(result.text).toBe('I am sorry to hear that.');
  });

  it('parses [angry] tag', () => {
    const result = parseTags("[angry] That's not right!");
    expect(result.emotion).toBe('angry');
  });

  it('parses [relaxed] tag', () => {
    const result = parseTags('[relaxed] Take it easy.');
    expect(result.emotion).toBe('relaxed');
  });

  it('parses [surprised] tag', () => {
    const result = parseTags('[surprised] Oh wow!');
    expect(result.emotion).toBe('surprised');
  });

  it('parses [neutral] tag', () => {
    const result = parseTags('[neutral] The weather is mild today.');
    expect(result.emotion).toBe('neutral');
  });

  it('returns original text when no tags', () => {
    const result = parseTags('Just plain text.');
    expect(result.emotion).toBeNull();
    expect(result.text).toBe('Just plain text.');
  });

  it('handles empty input', () => {
    const result = parseTags('');
    expect(result.emotion).toBeNull();
    expect(result.text).toBe('');
  });

  it('is case-insensitive for emotion tags', () => {
    const result = parseTags('[Happy] Hello!');
    expect(result.emotion).toBe('happy');
  });

  it('preserves unrecognized bracketed content', () => {
    const result = parseTags('[unknown] Some text.');
    expect(result.emotion).toBeNull();
    expect(result.text).toBe('[unknown] Some text.');
  });

  it('first emotion wins when multiple present', () => {
    const result = parseTags('[happy] [sad] Mixed feelings.');
    expect(result.emotion).toBe('happy');
    expect(result.text).toBe('Mixed feelings.');
  });

  it('handles tags with surrounding text', () => {
    const result = parseTags('Before [happy] after');
    expect(result.emotion).toBe('happy');
    expect(result.text).toBe('Before after');
  });

  it('handles tags at end of text', () => {
    const result = parseTags('Hello [happy]');
    expect(result.emotion).toBe('happy');
    expect(result.text).toBe('Hello');
  });

  it('unrecognized tags like pose are preserved in text', () => {
    const result = parseTags('[pose:sit] Hello!');
    expect(result.emotion).toBeNull();
    expect(result.text).toBe('[pose:sit] Hello!');
  });

  it('strips motion tags from text', () => {
    const result = parseTags('[motion:wave] Hello!');
    expect(result.emotion).toBeNull();
    expect(result.motion).toBe('wave');
    expect(result.text).toBe('Hello!');
  });

  it('strips motion:nod tag', () => {
    const result = parseTags('Sure! [motion:nod] I can help.');
    expect(result.motion).toBe('nod');
    expect(result.text).toBe('Sure! I can help.');
  });

  it('extracts both emotion and motion tags', () => {
    const result = parseTags('[happy] Hi there! [motion:wave] How can I help?');
    expect(result.emotion).toBe('happy');
    expect(result.motion).toBe('wave');
    expect(result.text).toBe('Hi there! How can I help?');
  });

  // ── <anim> block tests ──────────────────────────────────────────────────

  it('parses <anim> block with emotion', () => {
    const result = parseTags('<anim>{"emotion":"happy"}</anim>\nGreat to see you!');
    expect(result.emotion).toBe('happy');
    expect(result.text).toBe('Great to see you!');
  });

  it('parses <anim> block with motion', () => {
    const result = parseTags('<anim>{"motion":"wave"}</anim>\nHello!');
    expect(result.motion).toBe('wave');
    expect(result.text).toBe('Hello!');
  });

  it('parses <anim> block with both emotion and motion', () => {
    const result = parseTags('<anim>{"emotion":"surprised","motion":"nod"}</anim>\nWow!');
    expect(result.emotion).toBe('surprised');
    expect(result.motion).toBe('nod');
    expect(result.text).toBe('Wow!');
  });

  it('strips <anim> block with invalid JSON', () => {
    const result = parseTags('<anim>not json</anim>Hello!');
    expect(result.emotion).toBeNull();
    expect(result.text).toBe('Hello!');
  });

  it('prefers <anim> block emotion over legacy tag', () => {
    const result = parseTags('<anim>{"emotion":"happy"}</anim>[sad] Hello!');
    expect(result.emotion).toBe('happy');
    expect(result.text).toBe('Hello!');
  });
});

describe('emotion-parser — stripTags', () => {
  it('strips emotion tags', () => {
    expect(stripTags('[happy] Hello!')).toBe('Hello!');
  });

  it('returns original when no tags', () => {
    expect(stripTags('No tags here')).toBe('No tags here');
  });

  it('strips multiple emotion tags', () => {
    expect(stripTags('[happy] [sad] Great!')).toBe('Great!');
  });
});
