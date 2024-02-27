import { SoundManager } from './sound-manager.js'

function main() {
  const manager = new SoundManager()
  manager.addEventListener("soundloaded", ev => {
    // const file = ev.detail.file
  })
  manager.addEventListener("atlasloaded", () => {
    manager.loadEverything()
    updatePackageButtons(manager)
    manager.setPackageByName('localised')
    manager.setLanguage('english')
  })
  manager.addEventListener("packagechanged", () => {
    updatePackageButtons(manager)
    updateLanguageButtons(manager)
    updatePlayButtons(manager)
  })
  manager.addEventListener("languagechanged", () => {
    updateLanguageButtons(manager)
    updatePlayButtons(manager)
  })
  manager.loadAtlas('./encoded/.atlas.json')
  window.manager = manager
}

// ----------------- UI ----------------- //

/** @type {HTMLButtonElement[]} */
const playButtons = []
const playButtonContainer = document.getElementById('play-buttons')

/** @type {HTMLButtonElement[]} */
const languageButtons = []
const languageButtonContainer = document.getElementById('language-buttons')

/** @type {HTMLButtonElement[]} */
const packageButtons = []
const packageButtonContainer = document.getElementById('package-buttons')

/**
 * @param {HTMLButtonElement[]} buttons
 * @returns {HTMLButtonElement[]}
 * */
function clearButtons(buttons) {
  for (const button of buttons) {
    button.remove()
  }
  buttons.length = 0
  return buttons
}

/**
 * @param {SoundManager} manager
 * @returns {HTMLButtonElement[]}
 */
function updatePackageButtons(manager) {
  return clearButtons(packageButtons)
    .push(...
      manager.getPackageNames()
      .sort()
      .map(name => {
        const button = createButton(
          packageButtonContainer,
          name, () => {
            manager.setPackageByName(name)
          }
        )
        if (name === manager.cpn) {
          button.classList.add('selected')
        }
        return button
      }
    )
  )
}

/**
 * @param {SoundManager} manager
 * @returns {HTMLButtonElement[]}
 */
function updateLanguageButtons(manager) {
  return clearButtons(languageButtons)
    .push(...
      manager.languages()
        .sort()
        .map(language => {
          const button = createButton(
            languageButtonContainer,
            language, () => {
              manager.setLanguage(language)
            }
          )
          if (language === manager.language) {
            button.classList.add('selected')
          }
          return button
        })
    )
}

/**
 * @param {SoundManager} manager
 * @returns {HTMLButtonElement[]}
 */
function updatePlayButtons(manager) {
  return clearButtons(playButtons)
    .push(
      ...manager.sourceNames()
        .sort()
        .map(name => {
          /** @type {AudioBufferSourceNode | undefined} */
          let source
          const button = createButton(playButtonContainer, name, () => {
            if (button.classList.contains('playing')) {
              button.classList.remove('playing')
              try {
                source?.stop()
              } catch {}
              return
            }
            source = manager.context.createBufferSource()
            source.buffer = manager.requestBufferSync(name)
            source.connect(manager.context.destination)
            button.classList.add('playing')
            button.style.setProperty('--animation-duration', `${source?.buffer?.duration}s`)
            source.addEventListener('ended', () => {
              button.classList.remove('playing')
            })
            source.start()
          })
          return button
      })
    )
}

/**
 * @param {HTMLElement} parent
 * @param {string} name
 * @param {Function} onclick
 * @returns {HTMLButtonElement}
 */
function createButton(parent, name, onclick) {
  const button = document.createElement('button')
  button.textContent = name
  parent.appendChild(button)
  button.addEventListener('click', onclick)
  return button
}

main()
