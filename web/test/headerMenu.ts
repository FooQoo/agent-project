import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

/** Opens the header's menu and returns the item with this name (Docs, Epics, Archive or Settings). */
export async function headerMenuItem(name: string) {
  await userEvent.click(screen.getByRole('button', { name: /^(Menu|メニュー)$/ }));
  return screen.findByRole('menuitem', { name });
}

/** Chooses a header menu item, as a person does: open the menu, then pick the item. */
export async function chooseHeaderItem(name: string) {
  await userEvent.click(await headerMenuItem(name));
}
