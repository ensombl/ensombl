/**
 * Flags a node changed on disk: a flash when it just happened (keyed on the
 * timestamp so each new change replays it) and a dot until acknowledged.
 */
export function ChangeMarker({ changedAt }: { changedAt?: number }) {
  if (!changedAt) return null
  return (
    <>
      <span key={changedAt} className="crm-change-flash pointer-events-none absolute -inset-1 rounded-xl border-2 border-amber-400" />
      <span
        title="Changed since you last looked"
        className="pointer-events-none absolute -top-1 -right-1 size-2.5 rounded-full bg-amber-400 ring-2 ring-background"
      />
    </>
  )
}
