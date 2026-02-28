window.addEventListener("TrunkApplicationStarted", (_) => {
  console.log("wasm finished loading.")
  requestAnimationFrame(animationFrame);
})

const animation_frame_until_loaded = 2;
let animation_frame = 0;

function animationFrame() {
  animation_frame += 1;
  console.log("animation frame: ", animation_frame)
  if (animation_frame >= animation_frame_until_loaded) {
    document.getElementById("loading_screen").style.display = "none";
  } else {
    requestAnimationFrame(animationFrame);
  }
}

