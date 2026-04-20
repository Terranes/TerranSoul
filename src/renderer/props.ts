/**
 * Procedural "sitting with tea" props — sofa + teacup.
 *
 * These are generated from primitives (no external GLB) so the app ships
 * without extra assets.  When higher-fidelity GLB models are placed in
 * `public/models/props/` this module can be extended with a loadGLB path.
 */
import * as THREE from 'three';

export interface SittingProps {
  /** Group containing the sofa — add this to the scene and toggle `.visible`. */
  sofa: THREE.Group;
  /** Group containing the teacup — parented to the right hand bone at mount. */
  teacup: THREE.Group;
}

/** Create the sofa primitive — a low-poly two-seat couch in a warm neutral tone.
 *
 * Coordinate convention (matches the VRM scene):
 *  - +Z is toward the camera (viewer)
 *  - Character root is at origin; when sitting, the body is lowered by
 *    SITTING_BODY_Y_OFFSET (≈ -0.42m).
 *
 * We place the sofa so the seat cushion surface sits at y = 0.13 (just below
 * the lowered hip level) with the cushion extending forward to z = +0.45 so
 * the camera sees the couch beneath the character and the character reads
 * as sitting ON the sofa rather than standing behind it.
 */
function createSofa(): THREE.Group {
  const group = new THREE.Group();
  group.name = 'sitting-sofa';

  const fabricMat = new THREE.MeshStandardMaterial({
    color: 0x7a5a47,
    roughness: 0.85,
    metalness: 0.02,
  });
  const cushionMat = new THREE.MeshStandardMaterial({
    color: 0x9b7660,
    roughness: 0.85,
    metalness: 0.02,
  });
  const legMat = new THREE.MeshStandardMaterial({
    color: 0x2d221b,
    roughness: 0.55,
    metalness: 0.1,
  });

  // Geometry reference: cushion top at y = 0.17 (seat) + 0.06 (cushion half) = 0.20
  // Back of couch at z = -0.15 (behind character), front of cushion at z = +0.45
  // Arms flare outward so camera sees the couch silhouette on both sides.

  // Seat block (wide, shallow)
  const seat = new THREE.Mesh(new THREE.BoxGeometry(1.5, 0.16, 0.72), fabricMat);
  seat.position.set(0, 0.12, 0.12);
  group.add(seat);

  // Cushion on top of seat
  const cushion = new THREE.Mesh(new THREE.BoxGeometry(1.36, 0.10, 0.66), cushionMat);
  cushion.position.set(0, 0.25, 0.12);
  group.add(cushion);

  // Back rest — tall panel behind the character
  const back = new THREE.Mesh(new THREE.BoxGeometry(1.5, 0.7, 0.18), fabricMat);
  back.position.set(0, 0.55, -0.32);
  group.add(back);

  // Soft back cushion on top
  const backCushion = new THREE.Mesh(new THREE.BoxGeometry(1.36, 0.55, 0.14), cushionMat);
  backCushion.position.set(0, 0.58, -0.21);
  group.add(backCushion);

  // Arm rests — flare outward so the camera reads them clearly on both sides
  const armGeo = new THREE.BoxGeometry(0.18, 0.42, 0.72);
  const leftArm = new THREE.Mesh(armGeo, fabricMat);
  leftArm.position.set(-0.72, 0.36, 0.12);
  group.add(leftArm);
  const rightArm = new THREE.Mesh(armGeo, fabricMat);
  rightArm.position.set(0.72, 0.36, 0.12);
  group.add(rightArm);

  // Legs — four short cylinders at the corners, slightly visible under the seat
  const legGeo = new THREE.CylinderGeometry(0.04, 0.04, 0.12, 12);
  const legOffsets: [number, number][] = [
    [-0.62, -0.18],
    [0.62, -0.18],
    [-0.62, 0.40],
    [0.62, 0.40],
  ];
  for (const [x, z] of legOffsets) {
    const leg = new THREE.Mesh(legGeo, legMat);
    leg.position.set(x, 0.06, z);
    group.add(leg);
  }

  // Group root sits at the scene origin (floor).  The character is translated
  // down by SITTING_BODY_Y_OFFSET when the sitting idle is active, so their
  // hips land on the cushion at y ≈ 0.28 (cushion top).
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
 * Create both the sofa and teacup as independent groups.  The caller is
 * responsible for adding them to the scene (sofa) or parenting the teacup to
 * the character's right hand bone.  Both start hidden — call
 * setSittingPropsVisible(true) when the seated idle is active.
 */
export function createSittingProps(): SittingProps {
  const sofa = createSofa();
  const teacup = createTeacup();
  sofa.visible = false;
  teacup.visible = false;
  return { sofa, teacup };
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
