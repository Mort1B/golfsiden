import { decodeProfile, type ProfileDetails } from './profile'
import { jsonRequest, requestDecoded, requestNoContent } from './http'

export const profileApi = {
  get: (userId: string) => requestDecoded('/api/me/profile', (data) => decodeProfile(data, userId)),
  details: (userId: string, input: ProfileDetails, csrf: string) => requestDecoded('/api/me/profile',
    (data) => decodeProfile(data, userId), jsonRequest('PUT', input, csrf)),
  username: (version: number, username: string, current_password: string, csrf: string) => requestNoContent('/api/me/profile/username',
    jsonRequest('POST', { version, username, current_password }, csrf)),
  password: (version: number, new_password: string, current_password: string, csrf: string) => requestNoContent('/api/me/profile/password',
    jsonRequest('POST', { version, new_password, current_password }, csrf)),
}
