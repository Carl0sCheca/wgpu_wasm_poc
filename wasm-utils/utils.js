export async function read_json(path) {
    return (await fetch(path)).json();
}

export async function read_image(path) {
    return await (await fetch(path)).blob();
}