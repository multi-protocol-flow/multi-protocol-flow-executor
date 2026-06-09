export interface UpdateInfo {
  available: boolean
  currentVersion?: string
  version?: string
  date?: string
  body?: string
  error?: string
}

export interface DownloadEvent {
  event: 'Started' | 'Progress' | 'Finished'
  data?: { contentLength?: number; chunkLength?: number }
}
