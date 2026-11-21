/*
    SPDX-License-Identifier: MIT
    4Y Video Wallpaper — Plasma 6 wallpaper plugin.

    Plays a video directly inside plasmashell's desktop containment, so the
    desktop icons and their labels stay on top of the video (the old
    layer-shell based engine drew an opaque surface over the whole desktop,
    which hid them).

    Configuration is normally written by 4YWallPaper (the Tauri app) through
    Plasma's scripting interface:
        desktop.wallpaperPlugin = "org.foury.videowall"
        desktop.currentConfigGroup = ["Wallpaper", "org.foury.videowall", "General"]
        desktop.writeConfig("videoPath", "/path/to/video.mp4")
        desktop.writeConfig("fillMode", 1)
*/

import QtQuick
import QtMultimedia
import org.kde.plasma.plasmoid

WallpaperItem {
    id: root

    readonly property string videoPath: root.configuration.videoPath || ""
    readonly property url videoUrl: root.videoPath === "" ? "" : (root.videoPath.toString().indexOf("file://") === 0 ? root.videoPath : "file://" + root.videoPath)
    readonly property bool audioEnabled: root.configuration.audioEnabled ?? false
    readonly property real volumeLevel: root.configuration.volume ?? 0.5
    readonly property int fillModeIdx: root.configuration.fillMode ?? 1
    readonly property bool pausedByUser: root.configuration.paused ?? false

    // Fill modes: 0 = stretch, 1 = crop (cover), 2 = fit (contain)
    readonly property int effectiveFillMode: {
        if (fillModeIdx === 0)
            return VideoOutput.Stretch;
        if (fillModeIdx === 2)
            return VideoOutput.PreserveAspectFit;
        return VideoOutput.PreserveAspectCrop;
    }

    onVideoPathChanged: Qt.callLater(root.restart)
    onAudioEnabledChanged: Qt.callLater(root.applyAudio)
    onVolumeLevelChanged: Qt.callLater(root.applyAudio)
    onPausedByUserChanged: Qt.callLater(root.applyPaused)

    function restart() {
        if (root.videoPath === "") {
            player.stop();
            player.source = "";
            return;
        }
        if (player.source.toString() !== root.videoUrl.toString())
            player.source = root.videoUrl;
        if (!root.pausedByUser)
            player.play();
    }

    function applyAudio() {
        audioOut.muted = !root.audioEnabled;
        audioOut.volume = root.volumeLevel;
    }

    function applyPaused() {
        if (root.pausedByUser)
            player.pause();
        else
            player.play();
    }

    MediaPlayer {
        id: player
        loops: MediaPlayer.Infinite
        audioOutput: AudioOutput {
            id: audioOut
            muted: true
            volume: 0.5
        }
        videoOutput: out
        onErrorOccurred: (error, errorString) => console.warn("[4YWall] player error:", error, errorString)
        onMediaStatusChanged: {
            if (mediaStatus === MediaPlayer.InvalidMedia || mediaStatus === MediaPlayer.NoMedia)
                console.warn("[4YWall] cannot play", source);
        }
    }

    VideoOutput {
        id: out
        anchors.fill: parent
        fillMode: root.effectiveFillMode
        // Let the video fill the whole desktop area, under the icons.
        z: -1
    }

    // Keep the wallpaper from claiming focus / pointer input.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.NoButton
        propagateComposedEvents: true
        enabled: false
    }

    Component.onCompleted: {
        root.applyAudio();
        Qt.callLater(root.restart);
    }
}
