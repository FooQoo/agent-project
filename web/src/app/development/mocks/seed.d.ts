// The sample data of the browser mock, which `web/mock-browser.ts` reads from `web/test/fixtures/`.
declare module 'virtual:agpj-mock-seed' {
  export const fixture: import('@/shared/api/contracts').Board;
  export const longMarkdown: string;
}
