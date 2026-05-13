/* Minimal type declarations for d3-force-3d (no @types package exists). */
declare module 'd3-force-3d' {
  export interface SimulationNodeDatum {
    index?: number;
    x?: number;
    y?: number;
    z?: number;
    vx?: number;
    vy?: number;
    vz?: number;
    fx?: number | null;
    fy?: number | null;
    fz?: number | null;
  }

  export interface SimulationLinkDatum<N extends SimulationNodeDatum = SimulationNodeDatum> {
    source: N | number | string;
    target: N | number | string;
    index?: number;
  }

  export interface Force<N extends SimulationNodeDatum = SimulationNodeDatum> {
    (alpha: number): void;
    initialize?(nodes: N[], random: () => number): void;
  }

  export interface ForceLink<N extends SimulationNodeDatum, L extends SimulationLinkDatum<N>>
    extends Force<N> {
    links(): L[];
    links(links: L[]): this;
    id(): (node: N, i: number, nodes: N[]) => number | string;
    id(id: (node: N, i: number, nodes: N[]) => number | string): this;
    distance(): number | ((link: L, i: number, links: L[]) => number);
    distance(distance: number | ((link: L, i: number, links: L[]) => number)): this;
    strength(): number | ((link: L, i: number, links: L[]) => number);
    strength(strength: number | ((link: L, i: number, links: L[]) => number)): this;
  }

  export interface Simulation<
    N extends SimulationNodeDatum = SimulationNodeDatum,
    L extends SimulationLinkDatum<N> = SimulationLinkDatum<N>,
  > {
    restart(): this;
    stop(): this;
    tick(iterations?: number): this;
    nodes(): N[];
    nodes(nodes: N[]): this;
    alpha(): number;
    alpha(alpha: number): this;
    alphaMin(): number;
    alphaMin(min: number): this;
    alphaDecay(): number;
    alphaDecay(decay: number): this;
    alphaTarget(): number;
    alphaTarget(target: number): this;
    velocityDecay(): number;
    velocityDecay(decay: number): this;
    force(name: string): Force<N> | undefined;
    force(name: string, force: Force<N> | null): this;
    numDimensions(): number;
    numDimensions(n: 1 | 2 | 3): this;
    on(typenames: string, listener: ((this: Simulation<N, L>) => void) | null): this;
  }

  export function forceSimulation<N extends SimulationNodeDatum>(nodes?: N[]): Simulation<N>;
  export function forceLink<N extends SimulationNodeDatum, L extends SimulationLinkDatum<N>>(
    links?: L[],
  ): ForceLink<N, L>;
  export function forceManyBody<N extends SimulationNodeDatum>(): Force<N> & {
    strength(): number | ((d: N, i: number, data: N[]) => number);
    strength(strength: number | ((d: N, i: number, data: N[]) => number)): ForceCenter<N>;
  };
  export function forceCenter<N extends SimulationNodeDatum>(
    x?: number,
    y?: number,
    z?: number,
  ): ForceCenter<N>;

  export function forceCollide<N extends SimulationNodeDatum>(
    radius?: number | ((node: N, i: number, nodes: N[]) => number),
  ): Force<N> & {
    radius(): (node: N, i: number, nodes: N[]) => number;
    radius(radius: number | ((node: N, i: number, nodes: N[]) => number)): ForceCollide<N>;
    strength(): number;
    strength(strength: number): ForceCollide<N>;
    iterations(): number;
    iterations(iterations: number): ForceCollide<N>;
  };

  interface ForceCollide<N extends SimulationNodeDatum> extends Force<N> {
    radius(): (node: N, i: number, nodes: N[]) => number;
    radius(radius: number | ((node: N, i: number, nodes: N[]) => number)): this;
    strength(): number;
    strength(strength: number): this;
    iterations(): number;
    iterations(iterations: number): this;
  }

  interface ForceCenter<N extends SimulationNodeDatum> extends Force<N> {
    x(): number;
    x(x: number): this;
    y(): number;
    y(y: number): this;
    z(): number;
    z(z: number): this;
    strength(): number;
    strength(strength: number): this;
  }
}
