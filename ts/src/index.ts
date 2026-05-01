import { mountPelorusDial } from "./pelorus-dial";

function mount(): void {
  const host = document.getElementById("pelorus-dial-root");
  if (!host) {
    return;
  }
  mountPelorusDial(host);
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", mount);
} else {
  mount();
}
