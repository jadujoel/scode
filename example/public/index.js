import { NO_LANG, SoundManager } from './sound-manager.js';

function main() {
  const manager = new SoundManager();
  manager.addEventListener("soundloaded", ev => {
    const file = ev.detail.file;
    // do something with the file
  })
  manager.addEventListener("atlasloaded", () => {
    updatePackageButtons(manager);
  })
  manager.addEventListener("packagechanged", () => {
    // preload the non-localized sounds
    manager.loadLanguage(NO_LANG);
    updatePackageButtons(manager);
    updateLanguageButtons(manager);
    updatePlayButtons(manager);
  })
  manager.addEventListener("languagechanged", () => {
    manager.loadLanguage();
    updateLanguageButtons(manager);
    updatePlayButtons(manager);
  })
  manager.loadAtlas()
}

// ----------------- UI ----------------- //

/** @type {HTMLButtonElement[]} */
let playButtons = []
let playButtonContainer = document.getElementById('play-buttons');

/** @type {HTMLButtonElement[]} */
let languageButtons = [];
let languageButtonContainer = document.getElementById('language-buttons');

/** @type {HTMLButtonElement[]} */
let packageButtons = [];
let packageButtonContainer = document.getElementById('package-buttons');

function clearPlayButtons() {
  playButtons.forEach(button => {
    button?.remove()
  });
  playButtons = [];
}
function clearLanguageButtons() {
  languageButtons.forEach(button => {
    button?.remove()
  });
  languageButtons = [];
}

function clearPackageButtons() {
  packageButtons.forEach(button => {
    button?.remove()
  });
  packageButtons = [];
}

/**
 * @param {SoundManager} manager
 */
function updatePackageButtons(manager) {
  clearPackageButtons();
  packageButtons = manager.getPackages().sort().map(name => {
    const button = createButton(packageButtonContainer, name, () => {
      manager.setPackage(name);
    })
    if (name === manager.package) {
      button.classList.add('selected');
    }
    return button
  });
}

/**
 * @param {SoundManager} manager
 */
function updateLanguageButtons(manager) {
  clearLanguageButtons();
  languageButtons = manager.languages().sort().map(language => {
    const button = createButton(languageButtonContainer, language, () => {
      manager.setLanguage(language);
    })
    if (language === manager.language) {
      button.classList.add('selected');
    }
    return button
  });
}

/**
 * @param {SoundManager} manager
 */
function updatePlayButtons(manager) {
  clearPlayButtons();
  playButtons = manager.names().sort().map(name => {
    /** @type {AudioBufferSourceNode | undefined} */
    let source;
    const button = createButton(playButtonContainer, name, () => {
      if (button.classList.contains('playing')) {
        button.classList.remove('playing');
        try {
          source?.stop();
        } catch {}
        return;
      }
      source = manager.context.createBufferSource();
      source.buffer = manager.requestBufferSync(name);
      source.connect(manager.context.destination);

      button.classList.add('playing');
      button.style.setProperty('--animation-duration', `${source?.buffer?.duration}s`);
      source.addEventListener('ended', () => {
        button.classList.remove('playing');
      })

      source.start();
    })
    return button
  });
}

/**
 * @param {HTMLElement} parent
 * @param {string} name
 * @param {Function} onclick
 * @returns {HTMLButtonElement}
 */
function createButton(parent, name, onclick) {
  const button = document.createElement('button');
  button.textContent = name;
  parent.appendChild(button);
  button.addEventListener('click', onclick);
  return button;
}

main()
