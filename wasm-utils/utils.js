export const read_json = async (path) => (await fetch(path)).json();
export const read_image = async (path) => await (await fetch(path)).blob();
