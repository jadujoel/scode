import type { SoundManager } from 'smanager'
declare global {
    interface Window {
        manager: SoundManager
    }
}
