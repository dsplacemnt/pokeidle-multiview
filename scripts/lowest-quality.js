// Runs in the light background players. Opens the player's own settings → Quality and picks
// the lowest "NNNp" option, so hidden streams cost as little CPU/GPU and bandwidth as possible.
// It matches menu items by text (English and Portuguese); if the player's menu changes, it
// simply does nothing. Returns a promise that resolves to the chosen label, or null.
(() => {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const label = (el) =>
    (el.getAttribute('aria-label') || el.getAttribute('data-a-target') || el.textContent || '')
      .trim()
      .toLowerCase();
  const clickables = () => [
    ...document.querySelectorAll('button, [role="button"], [role="menuitem"], [role="menuitemradio"], label'),
  ];
  const find = (words) => clickables().find((el) => words.some((w) => label(el).includes(w)));
  const press = (el) => el.click();
  const height = (el) => Number((label(el).match(/(\d{3,4})p/) || [])[1] || NaN);

  return (async () => {
    for (let attempt = 0; attempt < 10; attempt++) {
      const gear = find(['settings', 'configura']);
      if (!gear) {
        await sleep(2000);
        continue;
      }
      press(gear);
      await sleep(600);
      const quality = find(['quality', 'qualidade']);
      if (!quality) {
        press(gear);
        await sleep(1000);
        continue;
      }
      press(quality);
      await sleep(600);
      const options = [...document.querySelectorAll('[role="menuitemradio"], [role="radio"], input[type="radio"], label')]
        .filter((el) => Number.isFinite(height(el)))
        .sort((a, b) => height(a) - height(b));
      if (!options.length) {
        press(gear);
        await sleep(1000);
        continue;
      }
      press(options[0]);
      await sleep(300);
      // Close the menu if it is still open.
      if (find(['quality', 'qualidade'])) press(gear);
      return label(options[0]);
    }
    return null;
  })();
})()
