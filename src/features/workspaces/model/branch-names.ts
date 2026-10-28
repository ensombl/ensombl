const FISH = [
  "graysby", "grouper", "wrasse", "tarpon", "snapper", "marlin", "barracuda", "halibut",
  "mackerel", "anchovy", "pollock", "tilefish", "lionfish", "sturgeon", "bonito", "sculpin",
  "goby", "blenny", "minnow", "darter", "pike", "perch", "smelt", "herring", "cobia", "wahoo",
  "dorado", "opah", "mullet", "triggerfish",
]

/** A friendly default branch name for a new worktree that doesn't clash with `taken`. */
export function suggestBranchName(taken: readonly string[], random = Math.random): string {
  const free = FISH.filter((f) => !taken.includes(f))
  if (free.length > 0) return free[Math.floor(random() * free.length)]
  let n = 2
  while (taken.includes(`${FISH[0]}-${n}`)) n++
  return `${FISH[0]}-${n}`
}
