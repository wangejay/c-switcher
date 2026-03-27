import { invoke } from "@tauri-apps/api/core";
import type { CurrentInfo, ProfileEntry, OpResult, UsageResult } from "./types";

export async function getCurrentInfo(): Promise<CurrentInfo> {
  return invoke("get_current_info");
}

export async function listProfiles(): Promise<ProfileEntry[]> {
  return invoke("list_profiles");
}

export async function backupProfile(name: string): Promise<OpResult> {
  return invoke("backup_profile", { name });
}

export async function switchProfile(name: string): Promise<OpResult> {
  return invoke("switch_profile", { name });
}

export async function deleteProfile(name: string): Promise<OpResult> {
  return invoke("delete_profile", { name });
}

export async function getUsage(
  profileName?: string | null
): Promise<UsageResult> {
  return invoke("get_usage", { profileName: profileName ?? null });
}

