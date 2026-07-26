import os
import time
from typing import Any

from proxmoxer import ProxmoxAPI


class ProxmoxSeeder:
    VMS = [
        {
            "vmid": 100,
            "name": "web-01",
            "cores": 2,
            "memory": 2048,
            "disk": "8",
            "tags": "production,web",
            "desc": "Web server frontend",
        },
        {
            "vmid": 101,
            "name": "web-02",
            "cores": 2,
            "memory": 2048,
            "disk": "8",
            "tags": "production,web",
            "desc": "Web server frontend",
        },
        {
            "vmid": 102,
            "name": "db-01",
            "cores": 4,
            "memory": 8192,
            "disk": "32",
            "tags": "production,database",
            "desc": "PostgreSQL primary",
        },
        {
            "vmid": 103,
            "name": "db-02",
            "cores": 4,
            "memory": 8192,
            "disk": "32",
            "tags": "production,database",
            "desc": "PostgreSQL replica",
        },
        {
            "vmid": 104,
            "name": "app-01",
            "cores": 2,
            "memory": 4096,
            "disk": "16",
            "tags": "production,app",
            "desc": "Application server",
        },
        {
            "vmid": 105,
            "name": "app-02",
            "cores": 2,
            "memory": 4096,
            "disk": "16",
            "tags": "production,app",
            "desc": "Application server",
        },
        {
            "vmid": 106,
            "name": "staging-web",
            "cores": 2,
            "memory": 2048,
            "disk": "8",
            "tags": "staging,web",
            "desc": "Staging web server",
        },
        {
            "vmid": 107,
            "name": "staging-db",
            "cores": 2,
            "memory": 4096,
            "disk": "16",
            "tags": "staging,database",
            "desc": "Staging database",
        },
        {
            "vmid": 108,
            "name": "staging-app",
            "cores": 2,
            "memory": 4096,
            "disk": "12",
            "tags": "staging,app",
            "desc": "Staging application",
        },
        {
            "vmid": 109,
            "name": "dev-workstation",
            "cores": 4,
            "memory": 8192,
            "disk": "40",
            "tags": "development,desktop",
            "desc": "Developer workstation",
        },
        {
            "vmid": 110,
            "name": "dev-test",
            "cores": 1,
            "memory": 1024,
            "disk": "4",
            "tags": "development,test",
            "desc": "CI test runner",
        },
        {
            "vmid": 111,
            "name": "win10",
            "cores": 4,
            "memory": 8192,
            "disk": "60",
            "tags": "desktop,windows",
            "desc": "Windows 10 desktop",
        },
    ]

    LXC_CONTAINERS = [
        {
            "vmid": 200,
            "hostname": "ct-proxy",
            "cores": 1,
            "memory": 512,
            "disk": "4",
            "tags": "production,proxy",
            "desc": "Nginx reverse proxy",
        },
        {
            "vmid": 201,
            "hostname": "ct-cache",
            "cores": 1,
            "memory": 1024,
            "disk": "8",
            "tags": "production,cache",
            "desc": "Redis cache server",
        },
        {
            "vmid": 202,
            "hostname": "ct-mq",
            "cores": 2,
            "memory": 2048,
            "disk": "8",
            "tags": "production,messaging",
            "desc": "RabbitMQ message queue",
        },
        {
            "vmid": 203,
            "hostname": "ct-monitor",
            "cores": 2,
            "memory": 4096,
            "disk": "16",
            "tags": "production,monitoring",
            "desc": "Prometheus + Grafana",
        },
        {
            "vmid": 204,
            "hostname": "ct-backup",
            "cores": 1,
            "memory": 1024,
            "disk": "32",
            "tags": "production,backup",
            "desc": "Backup server",
        },
        {
            "vmid": 205,
            "hostname": "ct-ansible",
            "cores": 2,
            "memory": 2048,
            "disk": "8",
            "tags": "development,ansible",
            "desc": "Ansible control node",
        },
    ]

    POOLS = [
        {"poolid": "production", "comment": "Production environment - customer facing"},
        {
            "poolid": "staging",
            "comment": "Staging environment - pre-release validation",
        },
        {
            "poolid": "development",
            "comment": "Development environment - internal testing",
        },
    ]

    POOL_VM_ASSIGNMENTS = {
        "production": [100, 101, 102, 103, 104, 105],
        "staging": [106, 107, 108],
        "development": [109, 110, 111],
    }

    SNAPSHOTS = [
        {"vmid": 100, "snapname": "pre-deploy", "desc": "Before v2.0 deployment"},
        {"vmid": 100, "snapname": "post-deploy", "desc": "After v2.0 deployment"},
        {"vmid": 102, "snapname": "baseline", "desc": "Clean database state"},
        {"vmid": 102, "snapname": "pre-migration", "desc": "Before schema migration"},
        {"vmid": 104, "snapname": "stable", "desc": "Known good version"},
        {"vmid": 106, "snapname": "test-start", "desc": "Initial staging test"},
        {"vmid": 109, "snapname": "dev-base", "desc": "Developer workstation base"},
    ]

    SDN_ZONES = [
        {"zone": "vlanz", "type": "vlan", "bridge": "vmbr0"},
    ]

    SDN_VNETS = [
        {"vnet": "prodnet", "zone": "vlanz", "tag": 100},
        {"vnet": "devnet", "zone": "vlanz", "tag": 200},
    ]

    REPLICATION_JOBS = [
        {"id": "100-0", "type": "local", "target": "pve", "schedule": "*/15"},
        {"id": "102-0", "type": "local", "target": "pve", "schedule": "02:00"},
    ]

    TASK_START_VMIDS = [103, 105, 107]

    HA_RESOURCES = [
        {"sid": "vm:100", "state": "started", "max_restart": 1, "max_relocate": 1},
        {"sid": "vm:102", "state": "started", "max_restart": 1, "max_relocate": 1},
        {"sid": "vm:104", "state": "stopped"},
    ]

    BACKUPS = [
        {
            "id": "backup-vm-100",
            "vmid": "100",
            "schedule": "02:00",
            "enabled": 1,
            "mode": "stop",
            "storage": "local",
        },
        {
            "id": "backup-vm-102",
            "vmid": "102",
            "schedule": "sun 03:00",
            "enabled": 1,
            "mode": "suspend",
            "storage": "local",
        },
    ]

    RUNNING_VMS = [100, 101, 102, 104, 106, 109, 111]

    def __init__(
        self,
        host: str | None = None,
        port: int | None = None,
        user: str | None = None,
        password: str | None = None,
        node: str | None = None,
    ) -> None:
        self.host = host or os.environ.get("PROXMOX_HOST", "localhost")
        self.port = port or int(os.environ.get("PROXMOX_PORT", "8006"))
        self.user = user or os.environ.get("PROXMOX_USER", "root@pam")
        self.password = password or os.environ.get("PROXMOX_PASSWORD", "root")
        self.node_name = node or os.environ.get("PROXMOX_NODE", "pve")

        self.pve: Any = None
        self.node: Any = None
        self.has_lxc_templates = False
        self.existing: dict[str, set] = {}

        self.created_vms: list = []
        self.created_cts: list = []
        self.created_sdn: list = []
        self.created_replication: list = []
        self.created_tasks: list = []
        self.created_ha: list = []
        self.created_backups: list = []
        self.disks: list = []

    @staticmethod
    def _create_if_missing(
        resource_id: str,
        existing: set,
        create_fn,
        created: list | None,
        resource_type: str,
    ) -> None:
        if resource_id in existing:
            print(f"  Skipping {resource_type} {resource_id}: already exists")
            return
        try:
            create_fn()
            print(f"  Created {resource_type}: {resource_id}")
            if created is not None:
                created.append(resource_id)
            existing.add(resource_id)
        except Exception as e:
            print(f"  Warning: {resource_type} {resource_id} issue: {e}")

    def connect(self, timeout: float = 300.0, interval: float = 5.0) -> None:
        print(f"\n[Wait] Waiting up to {int(timeout)}s for Proxmox to be ready...")
        deadline = time.monotonic() + timeout
        attempt = 0
        while True:
            attempt += 1
            try:
                self.pve = ProxmoxAPI(
                    self.host,
                    port=self.port,
                    user=self.user,
                    password=self.password,
                    verify_ssl=False,
                )
                self.node = self.pve.nodes(self.node_name)
                self.node.status.get()
                print(f"  Proxmox is ready (after {attempt} attempt(s))")
                return
            except Exception as e:
                if time.monotonic() >= deadline:
                    raise TimeoutError(
                        f"Proxmox not ready after {int(timeout)}s: {e}"
                    ) from e
                print(f"  Not ready yet (attempt {attempt}): {e}")
                time.sleep(interval)

    def check_storages(self) -> None:
        print("\n[Check] Available storages:")
        try:
            storages = self.node.storage.get()
            for s in storages:
                print(
                    f"  - {s.get('storage', 'unknown')} (type: {s.get('type', 'unknown')})"
                )
        except Exception as e:
            print(f"  Could not list storages: {e}")

    def check_templates(self) -> None:
        print("\n[Check] Available LXC templates:")
        try:
            templates = self.node.storage("local").content.get(content="vztmpl")
            if templates:
                for t in templates:
                    print(f"  - {t.get('volid', 'unknown')}")
            else:
                print("  No LXC templates found (LXC creation will be skipped)")
        except Exception as e:
            print(f"  Could not list templates: {e}")
            templates = []
        self.has_lxc_templates = len(templates) > 0

    def collect_existing_resources(self) -> None:
        print("\n[Check] Collecting existing resources...")
        self.existing = {
            "vmids": set(),
            "ctids": set(),
            "pools": set(),
            "sdn_zones": set(),
            "sdn_vnets": set(),
            "replication": set(),
            "ha": set(),
            "backups": set(),
        }
        try:
            self.existing["vmids"] = {vm.get("vmid") for vm in self.node.qemu.get()}
        except Exception as e:
            print(f"  Could not list VMs: {e}")
        try:
            self.existing["ctids"] = {ct.get("vmid") for ct in self.node.lxc.get()}
        except Exception as e:
            print(f"  Could not list containers: {e}")
        try:
            self.existing["pools"] = {
                pool.get("poolid") for pool in self.pve.pools.get()
            }
        except Exception as e:
            print(f"  Could not list pools: {e}")
        try:
            self.existing["sdn_zones"] = {
                zone.get("zone") for zone in self.pve.cluster.sdn.zones.get()
            }
        except Exception as e:
            print(f"  Could not list SDN zones: {e}")
        try:
            self.existing["sdn_vnets"] = {
                vnet.get("vnet") for vnet in self.pve.cluster.sdn.vnets.get()
            }
        except Exception as e:
            print(f"  Could not list SDN VNets: {e}")
        try:
            self.existing["replication"] = {
                job.get("id") for job in self.pve.cluster.replication.get()
            }
        except Exception as e:
            print(f"  Could not list replication jobs: {e}")
        try:
            self.existing["ha"] = {
                ha.get("sid") for ha in self.pve.cluster.ha.resources.get()
            }
        except Exception as e:
            print(f"  Could not list HA resources: {e}")
        try:
            self.existing["backups"] = {
                backup.get("id") for backup in self.pve.cluster.backup.get()
            }
        except Exception as e:
            print(f"  Could not list backup jobs: {e}")

    def create_vms(self) -> None:
        existing_vmids = self.existing["vmids"]
        for vm in self.VMS:
            if vm["vmid"] in existing_vmids:
                print(f"  Skipping VM {vm['vmid']}: already exists")
                continue
            try:
                self.node.qemu.create(
                    vmid=vm["vmid"],
                    name=vm["name"],
                    cores=vm["cores"],
                    memory=vm["memory"],
                    scsi0=f"local:{vm['disk']}",
                    net0="virtio,bridge=vmbr0",
                    ostype="l26",
                    tags=vm.get("tags", ""),
                    description=vm.get("desc", ""),
                    boot="order=scsi0",
                )
                print(
                    f"  Created VM {vm['vmid']}: {vm['name']} ({vm['cores']} cores, {vm['memory']}MB)"
                )
                self.created_vms.append(vm)
                existing_vmids.add(vm["vmid"])
                time.sleep(0.5)
            except Exception as e:
                print(f"  Warning: VM {vm['vmid']} creation issue: {e}")

    def create_lxc(self) -> None:
        if not self.has_lxc_templates:
            print("  Skipping LXC creation — no templates available")
            return
        existing_ctids = self.existing["ctids"]
        for ct in self.LXC_CONTAINERS:
            if ct["vmid"] in existing_ctids:
                print(f"  Skipping CT {ct['vmid']}: already exists")
                continue
            try:
                self.node.lxc.create(
                    vmid=ct["vmid"],
                    hostname=ct["hostname"],
                    cores=ct["cores"],
                    memory=ct["memory"],
                    rootfs=f"local:{ct['disk']}",
                    net0="name=eth0,bridge=vmbr0,ip=dhcp",
                    ostemplate="local:vztmpl/ubuntu-22.04-standard_22.04-1_amd64.tar.gz",
                    tags=ct.get("tags", ""),
                    description=ct.get("desc", ""),
                )
                print(
                    f"  Created CT {ct['vmid']}: {ct['hostname']} ({ct['cores']} cores, {ct['memory']}MB)"
                )
                self.created_cts.append(ct)
                existing_ctids.add(ct["vmid"])
                time.sleep(0.5)
            except Exception as e:
                print(f"  Warning: CT {ct['vmid']} creation issue: {e}")

    def create_pools(self) -> None:
        for pool in self.POOLS:
            self._create_if_missing(
                pool["poolid"],
                self.existing["pools"],
                lambda p=pool: self.pve.pools.create(
                    poolid=p["poolid"], comment=p["comment"]
                ),
                None,
                "pool",
            )

    def assign_pools(self) -> None:
        pool_assignments = {
            "production": self.POOL_VM_ASSIGNMENTS["production"]
            + [ct["vmid"] for ct in self.created_cts if ct["vmid"] <= 203],
            "staging": self.POOL_VM_ASSIGNMENTS["staging"]
            + [ct["vmid"] for ct in self.created_cts if ct["vmid"] == 204],
            "development": self.POOL_VM_ASSIGNMENTS["development"]
            + [ct["vmid"] for ct in self.created_cts if ct["vmid"] == 205],
        }
        for poolid, vmids in pool_assignments.items():
            try:
                members = self.pve.pools(poolid).get().get("members", [])
                assigned = {m.get("vmid") for m in members}
            except Exception:
                assigned = set()
            for vmid in vmids:
                if vmid in assigned:
                    print(f"  Skipping {vmid} -> {poolid}: already a member")
                    continue
                try:
                    self.pve.pools(poolid).put(vms=vmid)
                    print(f"  Assigned {vmid} -> {poolid}")
                except Exception as e:
                    print(f"  Warning: Assign {vmid} to {poolid} issue: {e}")

    def create_snapshots(self) -> None:
        for snap in self.SNAPSHOTS:
            try:
                self.node.qemu(snap["vmid"]).snapshot.create(
                    snapname=snap["snapname"],
                    description=snap["desc"],
                )
                print(f"  Created snapshot '{snap['snapname']}' on VM {snap['vmid']}")
            except Exception as e:
                print(
                    f"  Warning: Snapshot {snap['snapname']} on {snap['vmid']} issue: {e}"
                )

    def create_sdn(self) -> None:
        for zone in self.SDN_ZONES:
            self._create_if_missing(
                zone["zone"],
                self.existing["sdn_zones"],
                lambda z=zone: self.pve.cluster.sdn.zones.create(**z),
                self.created_sdn,
                "SDN zone",
            )
        for vnet in self.SDN_VNETS:
            self._create_if_missing(
                vnet["vnet"],
                self.existing["sdn_vnets"],
                lambda v=vnet: self.pve.cluster.sdn.vnets.create(**v),
                self.created_sdn,
                "SDN VNet",
            )
        try:
            self.pve.cluster.sdn.put()
            print("  Applied SDN configuration")
        except Exception as e:
            print(f"  Warning: SDN apply issue: {e}")

    def create_replication(self) -> None:
        try:
            node_count = len(self.pve.nodes.get())
        except Exception:
            node_count = 1
        if node_count < 2:
            print("  Skipping replication — requires at least 2 cluster nodes")
            return
        for job in self.REPLICATION_JOBS:
            self._create_if_missing(
                job["id"],
                self.existing["replication"],
                lambda j=job: self.pve.cluster.replication.create(**j),
                self.created_replication,
                "replication job",
            )

    def trigger_tasks(self) -> None:
        for vmid in self.TASK_START_VMIDS:
            try:
                task = self.node.qemu(vmid).status.start.post()
                print(f"  Triggered start task on VM {vmid}: {task}")
                self.created_tasks.append(task)
            except Exception as e:
                print(f"  Warning: Start task on VM {vmid} issue: {e}")

    def create_ha_resources(self) -> None:
        for ha in self.HA_RESOURCES:
            self._create_if_missing(
                ha["sid"],
                self.existing["ha"],
                lambda h=ha: self.pve.cluster.ha.resources.create(**h),
                self.created_ha,
                "HA resource",
            )

    def create_backups(self) -> None:
        for backup in self.BACKUPS:
            self._create_if_missing(
                backup["id"],
                self.existing["backups"],
                lambda b=backup: self.pve.cluster.backup.create(**b),
                self.created_backups,
                "backup job",
            )

    def list_node_disks(self) -> None:
        try:
            self.disks = self.node.disks.list.get()
            for disk in self.disks:
                print(
                    f"  Disk: {disk.get('devpath', 'unknown')} ({disk.get('model', 'unknown')})"
                )
        except Exception as e:
            print(f"  Warning: Could not list node disks: {e}")

    def start_vms(self) -> None:
        print("\n[Extra] Starting some VMs for realistic status...")
        for vmid in self.RUNNING_VMS:
            try:
                self.node.qemu(vmid).status.start.post()
                print(f"  Started VM {vmid}")
                time.sleep(1)
            except Exception as e:
                print(f"  Warning: Start VM {vmid} issue: {e}")

    def print_summary(self) -> None:
        print("\n" + "=" * 60)
        print("Fake Proxmox environment created successfully!")
        print("=" * 60)
        print(f"  VMs:          {len(self.created_vms)} / {len(self.VMS)} (qemu)")
        print(
            f"  Containers:   {len(self.created_cts)} / {len(self.LXC_CONTAINERS)} (lxc)"
        )
        print(f"  Pools:        {len(self.POOLS)}")
        print(f"  Snapshots:    {len(self.SNAPSHOTS)}")
        print(f"  SDN objects:  {len(self.created_sdn)}")
        print(f"  Replication:  {len(self.created_replication)}")
        print(f"  Tasks:        {len(self.created_tasks)}")
        print(f"  HA resources: {len(self.created_ha)}")
        print(f"  Backups:      {len(self.created_backups)}")
        print(f"  Disks:        {len(self.disks)}")
        print(f"  Running:      {len(self.RUNNING_VMS)} VMs started")
        print("\nYou can now connect with pvox:")
        print("  pvox --endpoint https://127.0.0.1:8006 --insecure")
        print("=" * 60)

    def run(self) -> None:
        print("=== Creating fake Proxmox environment ===")
        self.connect()
        self.check_storages()
        self.check_templates()
        self.collect_existing_resources()

        print("\n[1/11] Creating VMs...")
        self.create_vms()
        print("\n[2/11] Creating LXC containers...")
        self.create_lxc()
        print("\n[3/11] Creating pools...")
        self.create_pools()
        print("\n[4/11] Assigning resources to pools...")
        self.assign_pools()
        print("\n[5/11] Creating snapshots...")
        self.create_snapshots()
        print("\n[6/11] Creating SDN zones and VNets...")
        self.create_sdn()
        print("\n[7/11] Creating replication jobs...")
        self.create_replication()
        print("\n[8/11] Triggering cluster tasks...")
        self.trigger_tasks()
        print("\n[9/11] Creating HA resources...")
        self.create_ha_resources()
        print("\n[10/11] Creating backup jobs...")
        self.create_backups()
        print("\n[11/11] Listing node disks...")
        self.list_node_disks()
        self.start_vms()
        self.print_summary()


def main() -> None:
    ProxmoxSeeder().run()


if __name__ == "__main__":
    main()
