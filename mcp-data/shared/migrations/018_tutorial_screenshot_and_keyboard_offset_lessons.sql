-- 018_tutorial_screenshot_and_keyboard_offset_lessons.sql
-- Durable lessons from 2026-05-10 screenshot QA and mobile layout bugfix.

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Tutorial screenshot refresh must be captured and verified step-by-step, not by blind batch scripts. For each referenced tutorial image: open the exact target tab/view, dismiss quest overlays, confirm 3D mode state, capture, then visually verify the resulting file before moving to the next step. If any screenshot has UI defects (overlay, wrong mode, missing controls, clipping), fix UI/state first and immediately recapture that file. This avoids propagating one bad state across dozens of screenshots.',
  'lesson,tutorials,screenshots,qa,workflow,3d-mode,verification',
  10, 'procedure', 1746835200000, 'long', 1.0, 'self-improve', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Tutorial screenshot refresh must be captured and verified step-by-step%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'LESSON: Mobile black-strip and missing-input bug root cause was false keyboard detection in src/composables/useKeyboardDetector.ts. Resizing from 430x932 to 390x844 while the chat input remained focused set keyboardHeight=88 and translated .bottom-panel upward, leaving a dark gap and making the input appear missing. Durable fix: only treat keyboard as open when all are true: shrink > threshold, an editable element is focused, and visualViewport is actually reduced versus layout viewport (window.innerHeight - visualViewport.height > 20 or visualViewport.offsetTop > 0). Plain window/CDP resize must never trigger keyboard offset.',
  'lesson,frontend,mobile,keyboard,viewport,chat-input,layout,bugfix',
  10, 'procedure', 1746835200000, 'long', 1.0, 'frontend', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'LESSON: Mobile black-strip and missing-input bug root cause was false keyboard detection%'
);
