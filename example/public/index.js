const manager = window.manager
const playButtonContainer = document.getElementById("play-buttons");
const languageButtonContainer = document.getElementById("language-buttons");
const packageButtonContainer = document.getElementById("package-buttons");

manager.addEventListener("atlasloaded", () => {
  manager.activePackageNames = manager.getPackageNames();
  manager.setPackageByName("localised");
  manager.setLanguage("english");
});
manager.addEventListener("packagechanged", () => {
  updatePackageButtons();
  updateLanguageButtons();
  updatePlayButtons();
});
manager.addEventListener("languagechanged", () => {
  updateLanguageButtons();
  updatePlayButtons();
});

manager.loadAtlas("./encoded/.atlas.json");

function updatePackageButtons() {
  packageButtonContainer.innerHTML = "";
  return manager.getPackageNames().sort().map((name) => {
    const button = createButton(
      packageButtonContainer,
      name,
      () => {
        manager.setPackageByName(name);
      }
    );
    if (name === manager.cpn) {
      button.classList.add("selected");
    }
    return button;
  });
}
function updateLanguageButtons() {
  languageButtonContainer.innerHTML = "";
  return manager.getLanguages([manager.activePackageNames[0]]).sort().map((language) => {
    const button = createButton(
      languageButtonContainer,
      language,
      () => {
        manager.setLanguage(language);
      }
    );
    if (language === manager.language) {
      button.classList.add("selected");
    }
    return button;
  });
}
function updatePlayButtons() {
  playButtonContainer.innerHTML = "";
  return manager.getSourceNames(
    [manager.activePackageNames[0]],
    [manager.activeLanguages[0], "_"]
  ).sort().map((name) => {
    let source;
    const button = createButton(playButtonContainer, name, () => {
      if (source && button.classList.contains("playing")) {
        button.classList.remove("playing");
        source.stop();
        source = void 0;
        return;
      }
      source = manager.context.createBufferSource();
      source.buffer = manager.requestBufferSync(name);
      if (!source.buffer)
        return;
      source.connect(manager.context.destination);
      button.classList.add("playing");
      button.style.setProperty("--animation-duration", `${source.buffer.duration}s`);
      source.addEventListener("ended", () => {
        button.classList.remove("playing");
      });
      source.start();
    });
    return button;
  });
}
/** @type{(parent: HTMLElement, name: string, onclick: () => void) => HTMLButtonElement} */
function createButton(parent, name, onclick) {
  const button = document.createElement("button");
  button.textContent = name;
  button.addEventListener("click", () => onclick());
  parent.appendChild(button);
  return button;
}
