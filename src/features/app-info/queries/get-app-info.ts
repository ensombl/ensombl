import { useQuery } from "@tanstack/react-query"

import type { AppInfo } from "@/shared/bindings/AppInfo"
import { query } from "@/shared/ipc/client"

export const getAppInfo = () => query<AppInfo>("get_app_info")

export function useAppInfo() {
  return useQuery({ queryKey: ["app-info"], queryFn: getAppInfo, staleTime: Infinity })
}
