window.onload = function () {
    const player = new Plyr('video', {});

    // Expose player so it can be used from the console
    window.player = player;
}
