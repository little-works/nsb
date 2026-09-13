/** 组件类型 helper */
export function __render<
  P = {},
  E extends import('vue').EmitsOptions = {},
  S extends Record<string, any> = any,
>(fn: any): SetupFC<P, E, S> {
  return typeof fn === 'function' ? fn() : fn;
}
