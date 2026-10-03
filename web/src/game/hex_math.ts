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
  visibleHexes?: Set<string>
): number[] {
  const targets: number[] = [];
  for (const [idStr, u] of Object.entries(units)) {
    if (u.team !== ownTeam && u.hp > 0) {
      if (visibleHexes && !visibleHexes.has(`${u.pos.q},${u.pos.r}`)) {
        continue;
      }
      const dist = axialDistance(from, u.pos);
      if (dist <= range) {
        targets.push(Number(idStr));
      }
    }
  }
  return targets;
}
