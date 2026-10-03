import { HexCoord, UnitData } from './bridge';

export function axialDistance(a: HexCoord, b: HexCoord): number {
  return (Math.abs(a.q - b.q) + Math.abs(a.q + a.r - b.q - b.r) + Math.abs(a.r - b.r)) / 2;
}

export function axialNeighbors(h: HexCoord): HexCoord[] {
  return [
    { q: h.q + 1, r: h.r },
    { q: h.q + 1, r: h.r - 1 },
    { q: h.q, r: h.r - 1 },
    { q: h.q - 1, r: h.r },
    { q: h.q - 1, r: h.r + 1 },
    { q: h.q, r: h.r + 1 },
  ];
}

export function findPath(
  from: HexCoord,
  to: HexCoord,
  obstacles: Set<string>,
  walkable: Set<string>
): HexCoord[] | null {
  if (from.q === to.q && from.r === to.r) return [from];
  const toKey = `${to.q},${to.r}`;
  if (!walkable.has(toKey) || obstacles.has(toKey)) return null;

  const queue: HexCoord[] = [from];
  const visited = new Set<string>([`${from.q},${from.r}`]);
  const cameFrom = new Map<string, HexCoord>();

  while (queue.length > 0) {
    const current = queue.shift()!;
    if (current.q === to.q && current.r === to.r) {
      const path: HexCoord[] = [];
      let curr: HexCoord | undefined = current;
      while (curr) {
        path.push(curr);
        curr = cameFrom.get(`${curr.q},${curr.r}`);
      }
      path.reverse();
      return path;
    }

    for (const next of axialNeighbors(current)) {
      const key = `${next.q},${next.r}`;
      if (walkable.has(key) && !obstacles.has(key) && !visited.has(key)) {
        visited.add(key);
        cameFrom.set(key, current);
        queue.push(next);
      }
    }
  }

  return null;
}

export function getMoveTargets(
  unitPos: HexCoord,
  apBudget: number,
  obstacles: Set<string>,
  walkable: Set<string>,
  occupied: Set<string>
): { q: number; r: number; cost: number }[] {
  const result: { q: number; r: number; cost: number }[] = [];
  const distMap = new Map<string, number>();
  const queue: { hex: HexCoord; cost: number }[] = [{ hex: unitPos, cost: 0 }];
  distMap.set(`${unitPos.q},${unitPos.r}`, 0);

  while (queue.length > 0) {
    const { hex, cost } = queue.shift()!;
    if (cost < apBudget) {
      for (const next of axialNeighbors(hex)) {
        const nextKey = `${next.q},${next.r}`;
        const nextCost = cost + 1;
        if (!walkable.has(nextKey) || obstacles.has(nextKey) || occupied.has(nextKey)) {
          continue;
        }
        if (!distMap.has(nextKey) || nextCost < distMap.get(nextKey)!) {
          distMap.set(nextKey, nextCost);
          queue.push({ hex: next, cost: nextCost });
        }
      }
    }
  }

  distMap.forEach((cost, key) => {
    if (cost > 0 && cost <= apBudget) {
      const [q, r] = key.split(',').map(Number);
      result.push({ q, r, cost });
    }
  });

  return result;
}

export function getAttackTargets(
  from: HexCoord,
  range: number,
  units: Record<number, UnitData>,
  ownTeam: number,
  visibleHexes?: Set<string>,
  visionBlockers?: Set<string>
): number[] {
  const targets: number[] = [];
  for (const [idStr, u] of Object.entries(units)) {
    if (u.team !== ownTeam && u.hp > 0) {
      if (visibleHexes && !visibleHexes.has(`${u.pos.q},${u.pos.r}`)) {
        continue;
      }
      const dist = axialDistance(from, u.pos);
      if (dist <= range) {
        if (dist > 1 && visionBlockers && !hasLineOfSight(visionBlockers, from, u.pos)) {
          continue;
        }
        targets.push(Number(idStr));
      }
    }
  }
  return targets;
}

export function cubeRound(q: number, r: number, s: number): HexCoord {
  let rq = Math.round(q);
  let rr = Math.round(r);
  const rs = Math.round(s);

  const qDiff = Math.abs(rq - q);
  const rDiff = Math.abs(rr - r);
  const sDiff = Math.abs(rs - s);

  if (qDiff > rDiff && qDiff > sDiff) {
    rq = -rr - rs;
  } else if (rDiff > sDiff) {
    rr = -rq - rs;
  }

  return { q: rq, r: rr };
}

export function hexLine(a: HexCoord, b: HexCoord): HexCoord[] {
  const n = axialDistance(a, b);
  if (n === 0) return [a];

  const results: HexCoord[] = [];
  const aQ = a.q + 1e-6;
  const aR = a.r + 1e-6;
  const aS = -aQ - aR;

  const bQ = b.q + 2e-6;
  const bR = b.r + 2e-6;
  const bS = -bQ - bR;

  for (let i = 0; i <= n; i++) {
    const t = i / n;
    const q = aQ + (bQ - aQ) * t;
    const r = aR + (bR - aR) * t;
    const s = aS + (bS - aS) * t;
    results.push(cubeRound(q, r, s));
  }

  return results;
}

export function hasLineOfSight(
  visionBlockers: Set<string>,
  origin: HexCoord,
  target: HexCoord
): boolean {
  const line = hexLine(origin, target);
  for (let i = 1; i < line.length - 1; i++) {
    if (visionBlockers.has(`${line[i].q},${line[i].r}`)) {
      return false;
    }
  }
  return true;
}

export function getRepairTargets(
  from: HexCoord,
  units: Record<number, UnitData>,
  ownTeam: number
): number[] {
  const targets: number[] = [];
  for (const [idStr, u] of Object.entries(units)) {
    if (u.team === ownTeam && u.hp > 0 && u.hp < u.max_hp) {
      if (u.kind === 'Tower' || u.kind === 'Spawner' || u.kind === 'SpawnerTower') {
        const dist = axialDistance(from, u.pos);
        if (dist === 1) {
          targets.push(Number(idStr));
        }
      }
    }
  }
  return targets;
}

export interface SpellTargetingResult {
  validUnitIds: number[];
  obstructedUnitIds: number[];
  isSelfOnly: boolean;
}

export function getSpellTargets(
  caster: UnitData,
  spellId: string,
  units: Record<number, UnitData>,
  visionBlockers: Set<string>,
  visibleHexes?: Set<string>
): SpellTargetingResult {
  if (spellId === 'cleave') {
    return {
      validUnitIds: [caster.id],
      obstructedUnitIds: [],
      isSelfOnly: true,
    };
  }

  if (spellId === 'bolt') {
    const validUnitIds: number[] = [];
    const obstructedUnitIds: number[] = [];
    const range = 3;

    for (const [idStr, u] of Object.entries(units)) {
      if (u.team !== caster.team && u.hp > 0) {
        if (visibleHexes && !visibleHexes.has(`${u.pos.q},${u.pos.r}`)) {
          continue;
        }
        const dist = axialDistance(caster.pos, u.pos);
        if (dist >= 1 && dist <= range) {
          const id = Number(idStr);
          if (hasLineOfSight(visionBlockers, caster.pos, u.pos)) {
            validUnitIds.push(id);
          } else {
            obstructedUnitIds.push(id);
          }
        }
      }
    }

    return {
      validUnitIds,
      obstructedUnitIds,
      isSelfOnly: false,
    };
  }

  if (spellId === 'mend') {
    const validUnitIds: number[] = [];
    const obstructedUnitIds: number[] = [];
    const range = 2;

    for (const [idStr, u] of Object.entries(units)) {
      if (u.team === caster.team && u.hp > 0 && u.kind === 'Hero' && u.hp < u.max_hp) {
        const dist = axialDistance(caster.pos, u.pos);
        if (dist <= range) {
          const id = Number(idStr);
          if (dist === 0 || hasLineOfSight(visionBlockers, caster.pos, u.pos)) {
            validUnitIds.push(id);
          } else {
            obstructedUnitIds.push(id);
          }
        }
      }
    }

    return {
      validUnitIds,
      obstructedUnitIds,
      isSelfOnly: false,
    };
  }

  return { validUnitIds: [], obstructedUnitIds: [], isSelfOnly: false };
}

