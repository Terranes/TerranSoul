/**
 * Procedural sitting props — elegant chair + teacup.
 *
 * These are generated from primitives (no external GLB) so the app ships
 * without extra assets.  When higher-fidelity GLB models are placed in
 * `public/models/props/` this module can be extended with a loadGLB path.
 */
import * as THREE from 'three';

export interface SittingProps {
  /** Group containing the chair — add this to the scene and toggle `.visible`. */
  chair: THREE.Group;
  /** Group containing the teacup — parented to the right hand bone at mount. */
  teacup: THREE.Group;
}

/**
 * Create an elegant accent chair — mid-century modern style with tapered legs,
 * curved backrest, and plush upholstered cushion.
 *
 * Coordinate convention (matches the VRM scene):
 *  - +Z is toward the camera (viewer)
 *  - Character root is at origin
 *
 * The chair is sized and positioned so the seat surface sits just beneath
 * the character's hips during a VRMA sitting animation.  Tweak CHAIR_SEAT_Y
 * if the character floats above or sinks into the cushion.
 */

/** Seat surface Y — adjust to align with specific VRMA sitting animations. */
const CHAIR_SEAT_Y = 0.36;

function createChair(): THREE.Group {
  const group = new THREE.Group();
  group.name = 'sitting-chair';

  // ── Materials ──────────────────────────────────────────────────────
  const woodMat = new THREE.MeshStandardMaterial({
    color: 0x5c3a1e,      // rich walnut
    roughness: 0.45,
    metalness: 0.05,
  });
  const woodDarkMat = new THREE.MeshStandardMaterial({
    color: 0x3b2410,      // darker accent
    roughness: 0.40,
    metalness: 0.08,
  });
  const fabricMat = new THREE.MeshStandardMaterial({
    color: 0x4a3f5c,      // muted plum/mauve upholstery
    roughness: 0.82,
    metalness: 0.0,
  });
  const cushionMat = new THREE.MeshStandardMaterial({
    color: 0x584d6e,      // slightly lighter cushion highlight
    roughness: 0.78,
    metalness: 0.0,
  });

  // ── Seat frame (slightly rounded box) ─────────────────────────────
  const seatThickness = 0.06;
  const seatWidth = 0.52;
  const seatDepth = 0.48;
  const seatFrame = new THREE.Mesh(
    new THREE.BoxGeometry(seatWidth, seatThickness, seatDepth, 2, 1, 2),
    woodMat,
  );
  seatFrame.position.set(0, CHAIR_SEAT_Y - seatThickness / 2, 0.04);
  group.add(seatFrame);

  // ── Seat cushion (soft, slightly puffy) ───────────────────────────
  const cushionH = 0.055;
  const cushionGeo = new THREE.BoxGeometry(
    seatWidth - 0.04, cushionH, seatDepth - 0.04, 3, 2, 3,
  );
  // Slightly round the cushion vertices to give a pillow-like feel
  const pos = cushionGeo.attributes.position;
  for (let i = 0; i < pos.count; i++) {
    const x = pos.getX(i);
    const y = pos.getY(i);
    const z = pos.getZ(i);
    // Puff up the top face and round edges via a dome function
    const hw = (seatWidth - 0.04) / 2;
    const hd = (seatDepth - 0.04) / 2;
    const nx = x / hw;  // -1..1
    const nz = z / hd;  // -1..1
    const edgeFade = Math.max(0, 1 - nx * nx) * Math.max(0, 1 - nz * nz);
    if (y > 0) {
      pos.setY(i, y + edgeFade * 0.015);
    }
  }
  cushionGeo.computeVertexNormals();
  const cushion = new THREE.Mesh(cushionGeo, cushionMat);
  cushion.position.set(0, CHAIR_SEAT_Y + cushionH / 2, 0.04);
  group.add(cushion);

  // ── Backrest (curved panel) ───────────────────────────────────────
  const backWidth = seatWidth - 0.02;
  const backHeight = 0.42;
  const backThickness = 0.04;
  const backGeo = new THREE.BoxGeometry(
    backWidth, backHeight, backThickness, 4, 6, 1,
  );
  // Curve the backrest: arch the vertices in the Z direction
  const bPos = backGeo.attributes.position;
  for (let i = 0; i < bPos.count; i++) {
    const y = bPos.getY(i);
    const z = bPos.getZ(i);
    // Gentle concave curve — more pronounced at top
    const t = (y / (backHeight / 2)); // -1 at bottom, +1 at top
    const curvature = 0.04 * t * t;   // quadratic curve
    bPos.setZ(i, z - curvature);

    // Slight side-wing splay at top
    const x = bPos.getX(i);
    const wingFactor = Math.max(0, t) * 0.015;
    const sideSign = x > 0 ? 1 : x < 0 ? -1 : 0;
    bPos.setX(i, x + sideSign * wingFactor);
  }
  backGeo.computeVertexNormals();
  const backrest = new THREE.Mesh(backGeo, fabricMat);
  const backCenterY = CHAIR_SEAT_Y + backHeight / 2 + 0.02;
  backrest.position.set(0, backCenterY, -seatDepth / 2 + 0.02);
  group.add(backrest);

  // ── Backrest top rail (decorative wood strip) ─────────────────────
  const railGeo = new THREE.BoxGeometry(backWidth + 0.02, 0.028, 0.045, 4, 1, 1);
  // Match the backrest curve on the rail
  const rPos = railGeo.attributes.position;
  for (let i = 0; i < rPos.count; i++) {
    const x = rPos.getX(i);
    const z = rPos.getZ(i);
    const sideSign = x > 0 ? 1 : x < 0 ? -1 : 0;
    rPos.setX(i, x + sideSign * 0.015);
    // Slight forward arch
    const nx = x / ((backWidth + 0.02) / 2);
    rPos.setZ(i, z - 0.02 * nx * nx);
  }
  railGeo.computeVertexNormals();
  const rail = new THREE.Mesh(railGeo, woodDarkMat);
  rail.position.set(0, CHAIR_SEAT_Y + backHeight + 0.04, -seatDepth / 2 + 0.0);
  group.add(rail);

  // ── Legs (4 tapered cylinders, angled outward) ────────────────────
  const legH = CHAIR_SEAT_Y - seatThickness;
  const legTopR = 0.022;
  const legBotR = 0.016;
  const legGeo = new THREE.CylinderGeometry(legTopR, legBotR, legH, 10);

  // Leg positions: corners of the seat frame, slightly inset
  const legInset = 0.04;
  const legs: { x: number; z: number; tiltX: number; tiltZ: number }[] = [
    { x: -(seatWidth / 2 - legInset), z: -(seatDepth / 2 - legInset) + 0.04, tiltX: 0.06, tiltZ: -0.06 },   // back-left
    { x:  (seatWidth / 2 - legInset), z: -(seatDepth / 2 - legInset) + 0.04, tiltX: 0.06, tiltZ:  0.06 },   // back-right
    { x: -(seatWidth / 2 - legInset), z:  (seatDepth / 2 - legInset) + 0.04, tiltX: -0.06, tiltZ: -0.06 },  // front-left
    { x:  (seatWidth / 2 - legInset), z:  (seatDepth / 2 - legInset) + 0.04, tiltX: -0.06, tiltZ:  0.06 },  // front-right
  ];

  for (const l of legs) {
    const leg = new THREE.Mesh(legGeo, woodMat);
    leg.position.set(l.x, legH / 2, l.z);
    leg.rotation.set(l.tiltX, 0, l.tiltZ);
    group.add(leg);

    // Foot cap — small rounded sphere at the bottom of each leg
    const footGeo = new THREE.SphereGeometry(legBotR + 0.003, 8, 6, 0, Math.PI * 2, 0, Math.PI / 2);
    const foot = new THREE.Mesh(footGeo, woodDarkMat);
    foot.position.set(l.x + Math.sin(l.tiltZ) * legH * 0.5, 0.003, l.z - Math.sin(l.tiltX) * legH * 0.5);
    foot.rotation.x = Math.PI;
    group.add(foot);
  }

  // ── Back support struts (two vertical dowels connecting seat to backrest) ──
  const strutGeo = new THREE.CylinderGeometry(0.012, 0.012, backHeight * 0.6, 8);
  for (const xSign of [-1, 1]) {
    const strut = new THREE.Mesh(strutGeo, woodDarkMat);
    strut.position.set(
      xSign * (backWidth / 2 - 0.03),
      CHAIR_SEAT_Y + backHeight * 0.3,
      -seatDepth / 2 + 0.06,
    );
    group.add(strut);
  }

  group.position.set(0, 0, 0);
  return group;
}

/** Create the teacup primitive — saucer + cup + tiny handle. */
function createTeacup(): THREE.Group {
  const group = new THREE.Group();
  group.name = 'sitting-teacup';

  const porcelainMat = new THREE.MeshStandardMaterial({
    color: 0xf5f0e8,
    roughness: 0.35,
    metalness: 0.0,
  });
  const teaMat = new THREE.MeshStandardMaterial({
    color: 0x6b4423,
    roughness: 0.5,
    metalness: 0.0,
  });
  const rimMat = new THREE.MeshStandardMaterial({
    color: 0xc9a85a,
    roughness: 0.3,
    metalness: 0.25,
  });

  // Saucer — flat disc
  const saucer = new THREE.Mesh(
    new THREE.CylinderGeometry(0.055, 0.06, 0.008, 24),
    porcelainMat,
  );
  saucer.position.y = 0;
  group.add(saucer);

  // Cup body
  const cup = new THREE.Mesh(
    new THREE.CylinderGeometry(0.04, 0.032, 0.055, 24, 1, true),
    porcelainMat,
  );
  cup.position.y = 0.033;
  group.add(cup);

  // Bottom of cup (solid disc so the cup doesn't look hollow)
  const cupBase = new THREE.Mesh(
    new THREE.CircleGeometry(0.032, 24),
    porcelainMat,
  );
  cupBase.rotation.x = -Math.PI / 2;
  cupBase.position.y = 0.006;
  group.add(cupBase);

  // Tea liquid inside the cup
  const tea = new THREE.Mesh(
    new THREE.CircleGeometry(0.038, 24),
    teaMat,
  );
  tea.rotation.x = -Math.PI / 2;
  tea.position.y = 0.057;
  group.add(tea);

  // Gold rim
  const rim = new THREE.Mesh(
    new THREE.TorusGeometry(0.04, 0.0025, 8, 24),
    rimMat,
  );
  rim.rotation.x = Math.PI / 2;
  rim.position.y = 0.06;
  group.add(rim);

  // Handle
  const handle = new THREE.Mesh(
    new THREE.TorusGeometry(0.015, 0.004, 8, 16),
    porcelainMat,
  );
  handle.rotation.y = Math.PI / 2;
  handle.position.set(0.045, 0.033, 0);
  group.add(handle);

  return group;
}

/**
 * Create both the chair and teacup as independent groups.  The caller is
 * responsible for adding them to the scene (chair) or parenting the teacup to
 * the character's right hand bone.  Both start hidden — set `.visible = true`
 * when a sitting VRMA animation is active.
 */
export function createSittingProps(): SittingProps {
  const chair = createChair();
  const teacup = createTeacup();
  chair.visible = false;
  teacup.visible = false;
  return { chair, teacup };
}

/**
 * Tune the teacup's local offset relative to the right-hand bone so the cup
 * reads as being gripped naturally.  Values are in local bone space.
 */
export function applyTeacupHandOffset(teacup: THREE.Group) {
  // VRM right-hand bone: +X points outward along the palm, +Y up the forearm.
  // We tilt the cup slightly and slide it toward the fingertips so the rim
  // sits at lip height when the sitting pose raises the forearm.
  teacup.position.set(0.02, 0.01, 0.04);
  teacup.rotation.set(0, 0, -0.15);
  teacup.scale.setScalar(1.0);
}

/** Fade sofa or teacup opacity — call each frame while visible state is
 *  transitioning.  Uses material transparency on all mesh children. */
export function setPropGroupOpacity(group: THREE.Group, opacity: number) {
  group.traverse((obj) => {
    if ((obj as THREE.Mesh).isMesh) {
      const mat = (obj as THREE.Mesh).material as THREE.Material | THREE.Material[];
      const apply = (m: THREE.Material) => {
        m.transparent = opacity < 1;
        (m as THREE.MeshStandardMaterial).opacity = opacity;
      };
      if (Array.isArray(mat)) mat.forEach(apply);
      else apply(mat);
    }
  });
}
