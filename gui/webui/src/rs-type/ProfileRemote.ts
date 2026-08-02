// This file mirrors the Rust ProfileRemote type.
import type { ProfileHeader } from './ProfileHeader';

export type ProfileRemote = {
  name: string;
  url: string;
  headers: Array<ProfileHeader>;
};
