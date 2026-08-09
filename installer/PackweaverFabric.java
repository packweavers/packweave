package packweaver;

import net.fabricmc.loader.api.entrypoint.PreLaunchEntrypoint;

public class PackweaverFabric implements PreLaunchEntrypoint {
    @Override
    public void onPreLaunch() {
        Packweaver.modEntry();
    }
}
