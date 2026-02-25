package.preload["lush.audio"] = function()
  local audio = {}

  function audio.set_volume(percent)
    return _lush_audio_set_volume(math.max(0, math.floor((percent or 0) + 0.5)))
  end

  function audio.toggle_mute()
    return _lush_audio_toggle_mute()
  end

  function audio.set_muted(muted)
    return _lush_audio_set_muted(muted and true or false)
  end

  return audio
end
