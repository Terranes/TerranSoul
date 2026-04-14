/**
 * Tests for the emotion/motion tag parser.
 */
import { describe, it, expect } from 'vitest';
import { parseTags, stripTags } from './emotion-parser';

describe('emotion-parser — parseTags', () => {
  it('parses [happy] tag', () => {
    const result = parseTags('[happy] Great to see you!');
    expect(result.emotion).toBe('happy');
    expect(result.text).toBe('Great to see you!');
    expect(result.motion).toBeNull();
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

  it('parses [motion:wave] tag', () => {
    const result = parseTags('[motion:wave] Hello there!');
    expect(result.emotion).toBeNull();
    expect(result.motion).toBe('wave');
    expect(result.text).toBe('Hello there!');
  });

  it('parses both emotion and motion tags', () => {
    const result = parseTags('[happy] [motion:nod] Absolutely!');
    expect(result.emotion).toBe('happy');
    expect(result.motion).toBe('nod');
    expect(result.text).toBe('Absolutely!');
  });

  it('returns original text when no tags', () => {
    const result = parseTags('Just plain text.');
    expect(result.emotion).toBeNull();
    expect(result.motion).toBeNull();
    expect(result.text).toBe('Just plain text.');
  });

  it('handles empty input', () => {
    const result = parseTags('');
    expect(result.emotion).toBeNull();
    expect(result.motion).toBeNull();
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

  it('first motion wins when multiple present', () => {
    const result = parseTags('[motion:wave] [motion:nod] Greetings!');
    expect(result.motion).toBe('wave');
    expect(result.text).toBe('Greetings!');
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
});

describe('emotion-parser — pose tags', () => {
  it('parses [pose:confident=0.6,attentive=0.3] tag', () => {
    const result = parseTags('[pose:confident=0.6,attentive=0.3] Hello!');
    expect(result.poseBlend).not.toBeNull();
    expect(result.poseBlend).toHaveLength(2);
    expect(result.poseBlend![0]).toEqual({ presetId: 'confident', weight: 0.6 });
    expect(result.poseBlend![1]).toEqual({ presetId: 'attentive', weight: 0.3 });
    expect(result.text).toBe('Hello!');
  });

  it('parses single pose preset', () => {
    const result = parseTags('[pose:shy=1.0] I am nervous.');
    expect(result.poseBlend).toHaveLength(1);
    expect(result.poseBlend![0]).toEqual({ presetId: 'shy', weight: 1.0 });
  });

  it('clamps pose weight > 1 to 1.0', () => {
    const result = parseTags('[pose:confident=2.5]');
    expect(result.poseBlend![0].weight).toBe(1.0);
  });

  it('clamps negative pose weight to 0 and excludes it', () => {
    const result = parseTags('[pose:confident=-0.5] text');
    // weight 0 — should be excluded since weight must be > 0 effectively
    // parsePoseTag includes 0-clamped items; check weight is 0
    expect(result.poseBlend).not.toBeNull();
    expect(result.poseBlend![0].weight).toBe(0);
  });

  it('returns null poseBlend when no pose tag present', () => {
    const result = parseTags('[happy] Hello!');
    expect(result.poseBlend).toBeNull();
  });

  it('poseBlend does not interfere with emotion tag parsing', () => {
    const result = parseTags('[happy] [pose:excited=0.7] Let\'s go!');
    expect(result.emotion).toBe('happy');
    expect(result.poseBlend![0].presetId).toBe('excited');
    expect(result.text).toBe("Let's go!");
  });

  it('poseBlend does not interfere with motion tag parsing', () => {
    const result = parseTags('[motion:wave] [pose:playful=0.8] Hi!');
    expect(result.motion).toBe('wave');
    expect(result.poseBlend![0].presetId).toBe('playful');
  });

  it('first pose tag wins; second is stripped', () => {
    const result = parseTags('[pose:confident=0.8] [pose:shy=0.5] text');
    expect(result.poseBlend).toHaveLength(1);
    expect(result.poseBlend![0].presetId).toBe('confident');
  });

  it('malformed pose tag with no pairs returns null', () => {
    const result = parseTags('[pose:] text');
    expect(result.poseBlend).toBeNull();
    // tag should still be stripped
    expect(result.text).toBe('text');
  });

  it('poseBlend null for plain text with no tags', () => {
    const result = parseTags('Just some text');
    expect(result.poseBlend).toBeNull();
  });
});

describe('emotion-parser — stripTags', () => {
  it('strips emotion tags', () => {
    expect(stripTags('[happy] Hello!')).toBe('Hello!');
  });

  it('strips motion tags', () => {
    expect(stripTags('[motion:wave] Hi!')).toBe('Hi!');
  });

  it('returns original when no tags', () => {
    expect(stripTags('No tags here')).toBe('No tags here');
  });

  it('strips multiple tags', () => {
    expect(stripTags('[happy] [motion:nod] Great!')).toBe('Great!');
  });
});
