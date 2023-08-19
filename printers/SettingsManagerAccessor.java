package mixin;

import carpet.settings.SettingsManager;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

@Mixin(SettingsManager.class)
public interface SettingsManagerAccessor {
    @Accessor(value = "identifier", remap = false)
    String getIdentifier();
}
