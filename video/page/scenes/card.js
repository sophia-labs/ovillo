// Title and end cards: the Ovillo mark (an egg made out of one string, from
// shrubbery apps/ovillo/public/ovillo-mark.svg, copied) drawing itself, and
// plain text. Everything is a function of the card's local time.
const MARK = 'M22.8 21.4C22.4 21.5 21.2 22.1 20.2 22.1C19.2 22.1 18.1 21.2 16.8 21.2C15.5 21.2 13.9 22.2 12.4 22.3C10.9 22.4 8.9 22.0 8.0 21.6C7.1 21.2 6.7 20.3 6.9 19.7C7.1 19.1 8.4 18.2 9.4 17.9C10.4 17.6 12.1 18.1 13.1 17.9C13.5 17.7 14.8 17.1 15.5 16.5C16.2 15.8 16.7 15.0 17.1 14.1C17.4 13.2 17.5 12.1 17.5 11.1C17.4 10.1 17.2 9.0 16.9 8.0C16.5 7.0 16.0 6.0 15.5 5.2C14.9 4.4 14.3 3.7 13.6 3.2C13.0 2.7 12.3 2.4 11.6 2.3C10.9 2.1 10.2 2.2 9.5 2.5C8.8 2.8 8.2 3.3 7.5 3.9C6.9 4.6 6.3 5.4 5.9 6.3C5.4 7.2 5.0 8.3 4.8 9.3C4.5 10.3 4.4 11.4 4.6 12.4C4.7 13.4 5.0 14.4 5.5 15.2C5.9 16.0 6.7 16.7 7.4 17.2C8.2 17.7 9.2 18.0 10.2 18.1C11.1 18.3 12.6 17.9 13.1 17.9C12.6 17.9 11.3 18.2 10.4 18.2C9.5 18.1 8.3 17.6 7.9 17.4C7.8 17.3 7.4 17.0 7.4 16.5C7.4 16.1 7.6 15.4 8.0 14.7C8.4 14.1 9.1 13.2 9.7 12.6C10.3 12.0 10.8 11.5 11.4 11.1C12.0 10.6 12.6 10.2 13.3 9.9C13.9 9.5 14.8 9.2 15.3 9.1C15.9 9.1 16.5 9.2 16.8 9.4C17.2 9.6 17.3 10.2 17.4 10.3C17.3 10.0 17.2 8.9 16.9 8.2C16.7 7.5 16.2 6.5 16.1 6.2C15.9 6.1 15.5 5.6 15.1 5.5C14.7 5.4 14.2 5.4 13.7 5.5C13.1 5.6 12.5 5.9 11.8 6.3C11.2 6.6 10.5 7.2 9.8 7.7C9.1 8.2 8.4 8.9 7.8 9.5C7.2 10.2 6.6 10.9 6.2 11.5C5.8 12.1 5.4 12.8 5.2 13.3C5.1 13.8 5.2 14.5 5.2 14.7C5.1 14.3 4.7 13.4 4.6 12.6C4.5 11.9 4.6 10.8 4.6 10.4C4.6 10.2 4.8 9.7 5.0 9.3C5.2 8.9 5.5 8.5 5.8 8.0C6.2 7.5 6.9 6.7 7.4 6.2C7.8 5.7 8.2 5.4 8.7 5.0C9.1 4.6 9.5 4.3 10.0 4.0C10.5 3.7 11.2 3.3 11.7 3.1C12.1 3.0 12.4 2.9 12.7 2.9C13.0 2.9 13.4 3.1 13.5 3.1';

let built = null;
let pathLength = 0;
const ease = (x) => (x <= 0 ? 0 : x >= 1 ? 1 : 1 - (1 - x) ** 3);
const fade = (t, at, dur = 0.35) => ease((t - at) / dur);

function build(host, seg) {
  const svg = `<svg class="mark" viewBox="1.5 0.6 23 23" xmlns="http://www.w3.org/2000/svg">
    <path class="halo" d="${MARK}" fill="none" stroke="#fda4af" stroke-opacity="0.14" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>
    <path class="core" d="${MARK}" fill="none" stroke="#fda4af" stroke-width="0.95" stroke-linecap="round" stroke-linejoin="round"/>
  </svg>`;
  if (seg.id === 'title') {
    host.innerHTML = `${svg}<div class="word">${seg.lines[0]}</div><div class="tag">${seg.lines[1]}</div><div class="rule">${seg.lines[2]}</div>`;
  } else {
    host.innerHTML = `${svg}<div class="url main">${seg.lines[0]}</div><div class="url">${seg.lines[1]}</div><div class="lic">${seg.lines[2]}</div>`;
  }
  pathLength = host.querySelector('path.core').getTotalLength();
  built = seg.id;
}

export function renderCard(host, seg, t, duration) {
  if (built !== seg.id) build(host, seg);
  // frame 0 already shows the word and part of the thread (a feed thumbnail is never black)
  const draw = seg.id === 'title' ? 0.32 + 0.68 * ease(t / 1.1) : 1;
  for (const p of host.querySelectorAll('path')) {
    p.style.strokeDasharray = `${pathLength} ${pathLength}`;
    p.style.strokeDashoffset = `${pathLength * (1 - draw)}`;
  }
  const kids = [...host.children].slice(1);
  const at = seg.id === 'title' ? [-1, 0.4, 0.85] : [-1, -1, 0.3];
  kids.forEach((el, i) => { el.style.opacity = String(fade(t, at[i])); });
  // the end card dims a little in its last half second (a tail before the loop, never black)
  host.style.opacity = seg.id === 'end' ? String(1 - 0.4 * ease((t - (duration - 0.5)) / 0.5)) : '1';
}
