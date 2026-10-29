import { useMutation, useQuery } from "@tanstack/react-query"
import { relaunch } from "@tauri-apps/plugin-process"
import { check, type Update } from "@tauri-apps/plugin-updater"

const SIX_HOURS = 6 * 60 * 60 * 1000

export const updateKeys = { check: ["updates", "check"] as const }

/** The newer release, if any. Checked on start and every few hours. */
export function useAvailableUpdate() {
  return useQuery<Update | null>({
    queryKey: updateKeys.check,
    queryFn: () => check(),
    refetchInterval: SIX_HOURS,
    refetchOnWindowFocus: false,
    retry: false,
    staleTime: SIX_HOURS,
  })
}

/** Downloads and installs the update, then restarts into it. */
export function useInstallUpdate() {
  return useMutation({
    mutationFn: async (update: Update) => {
      await update.downloadAndInstall()
      await relaunch()
    },
  })
}
