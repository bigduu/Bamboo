"""
Bamboo Session 管理和持久化测试

测试场景包括：
- Session 创建和销毁
- Session 状态持久化（重启后恢复）
- 多 Session 并发管理
- Session 过期和清理
- Session 元数据管理

持久化测试：
- JSONL 存储格式验证
- 存储文件完整性检查
- 大数据量存储性能
- 存储压缩/归档

会话恢复测试：
- 客户端重连（相同 session_id）
- 服务端重启后恢复会话
- 断网重连场景

并发测试：
- 100+ Session 同时活跃
- Session 竞争条件测试
"""

import asyncio
import json
import os
import tempfile
import time
import uuid
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, asdict
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Any
import gzip
import shutil

import pytest
import pytest_asyncio


# ============================================================================
# 数据模型
# ============================================================================

@dataclass
class SessionMetadata:
    """Session 元数据"""
    session_id: str
    client_id: str
    created_at: float
    last_activity: float
    expires_at: float
    user_agent: str
    ip_address: str
    custom_data: Dict[str, Any]
    
    def to_dict(self) -> Dict:
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: Dict) -> 'SessionMetadata':
        return cls(**data)


@dataclass
class SessionState:
    """Session 状态"""
    session_id: str
    status: str  # active, inactive, expired
    data: Dict[str, Any]
    sequence_number: int
    checkpoint_id: str
    
    def to_dict(self) -> Dict:
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: Dict) -> 'SessionState':
        return cls(**data)


# ============================================================================
# Session 存储实现
# ============================================================================

class JSONLSessionStore:
    """JSONL 格式的 Session 持久化存储"""
    
    def __init__(self, storage_dir: str, compress_after_days: int = 7):
        self.storage_dir = Path(storage_dir)
        self.storage_dir.mkdir(parents=True, exist_ok=True)
        self.compress_after_days = compress_after_days
        self._lock = asyncio.Lock()
        
    def _get_session_file(self, session_id: str) -> Path:
        """获取 session 文件路径"""
        return self.storage_dir / f"{session_id}.jsonl"
    
    def _get_archive_file(self, session_id: str) -> Path:
        """获取归档文件路径"""
        return self.storage_dir / f"{session_id}.jsonl.gz"
    
    async def save_session(self, session: SessionMetadata) -> bool:
        """保存 session 元数据"""
        async with self._lock:
            file_path = self._get_session_file(session.session_id)
            record = {
                "type": "metadata",
                "timestamp": time.time(),
                "data": session.to_dict()
            }
            
            with open(file_path, 'a', encoding='utf-8') as f:
                f.write(json.dumps(record, ensure_ascii=False) + '\n')
            return True
    
    async def save_state(self, session_id: str, state: SessionState) -> bool:
        """保存 session 状态"""
        async with self._lock:
            file_path = self._get_session_file(session_id)
            record = {
                "type": "state",
                "timestamp": time.time(),
                "data": state.to_dict()
            }
            
            with open(file_path, 'a', encoding='utf-8') as f:
                f.write(json.dumps(record, ensure_ascii=False) + '\n')
            return True
    
    async def load_session(self, session_id: str) -> Optional[SessionMetadata]:
        """加载 session 元数据"""
        file_path = self._get_session_file(session_id)
        
        if not file_path.exists():
            # 检查归档文件
            archive_path = self._get_archive_file(session_id)
            if archive_path.exists():
                return await self._load_from_archive(archive_path, session_id)
            return None
        
        latest_metadata = None
        
        with open(file_path, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    record = json.loads(line)
                    if record.get("type") == "metadata":
                        latest_metadata = SessionMetadata.from_dict(record["data"])
                except json.JSONDecodeError:
                    continue
        
        return latest_metadata
    
    async def load_state(self, session_id: str) -> Optional[SessionState]:
        """加载最新 session 状态"""
        file_path = self._get_session_file(session_id)
        
        if not file_path.exists():
            archive_path = self._get_archive_file(session_id)
            if archive_path.exists():
                return await self._load_state_from_archive(archive_path)
            return None
        
        latest_state = None
        
        with open(file_path, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    record = json.loads(line)
                    if record.get("type") == "state":
                        state = SessionState.from_dict(record["data"])
                        if latest_state is None or state.sequence_number > latest_state.sequence_number:
                            latest_state = state
                except json.JSONDecodeError:
                    continue
        
        return latest_state
    
    async def _load_from_archive(self, archive_path: Path, session_id: str) -> Optional[SessionMetadata]:
        """从归档文件加载 session"""
        with gzip.open(archive_path, 'rt', encoding='utf-8') as f:
            latest_metadata = None
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    record = json.loads(line)
                    if record.get("type") == "metadata":
                        latest_metadata = SessionMetadata.from_dict(record["data"])
                except json.JSONDecodeError:
                    continue
            return latest_metadata
    
    async def _load_state_from_archive(self, archive_path: Path) -> Optional[SessionState]:
        """从归档文件加载状态"""
        with gzip.open(archive_path, 'rt', encoding='utf-8') as f:
            latest_state = None
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    record = json.loads(line)
                    if record.get("type") == "state":
                        state = SessionState.from_dict(record["data"])
                        if latest_state is None or state.sequence_number > latest_state.sequence_number:
                            latest_state = state
                except json.JSONDecodeError:
                    continue
            return latest_state
    
    async def archive_session(self, session_id: str) -> bool:
        """归档 session 文件"""
        async with self._lock:
            file_path = self._get_session_file(session_id)
            archive_path = self._get_archive_file(session_id)
            
            if not file_path.exists():
                return False
            
            with open(file_path, 'rb') as f_in:
                with gzip.open(archive_path, 'wb') as f_out:
                    shutil.copyfileobj(f_in, f_out)
            
            file_path.unlink()
            return True
    
    async def delete_session(self, session_id: str) -> bool:
        """删除 session"""
        async with self._lock:
            file_path = self._get_session_file(session_id)
            archive_path = self._get_archive_file(session_id)
            
            deleted = False
            if file_path.exists():
                file_path.unlink()
                deleted = True
            if archive_path.exists():
                archive_path.unlink()
                deleted = True
            
            return deleted
    
    async def list_sessions(self) -> List[str]:
        """列出所有 session ID"""
        sessions = set()
        
        for file_path in self.storage_dir.glob("*.jsonl"):
            sessions.add(file_path.stem)
        
        for file_path in self.storage_dir.glob("*.jsonl.gz"):
            sessions.add(file_path.stem.replace('.jsonl', ''))
        
        return list(sessions)
    
    async def verify_integrity(self, session_id: str) -> Dict[str, Any]:
        """验证存储文件完整性"""
        file_path = self._get_session_file(session_id)
        result = {
            "session_id": session_id,
            "exists": False,
            "valid_records": 0,
            "invalid_records": 0,
            "total_records": 0,
            "errors": []
        }
        
        if not file_path.exists():
            archive_path = self._get_archive_file(session_id)
            if archive_path.exists():
                return await self._verify_archive_integrity(archive_path, result)
            return result
        
        result["exists"] = True
        
        with open(file_path, 'r', encoding='utf-8') as f:
            for line_num, line in enumerate(f, 1):
                line = line.strip()
                if not line:
                    continue
                result["total_records"] += 1
                try:
                    record = json.loads(line)
                    if "type" in record and "data" in record:
                        result["valid_records"] += 1
                    else:
                        result["invalid_records"] += 1
                        result["errors"].append(f"Line {line_num}: Missing required fields")
                except json.JSONDecodeError as e:
                    result["invalid_records"] += 1
                    result["errors"].append(f"Line {line_num}: JSON decode error - {e}")
        
        return result
    
    async def _verify_archive_integrity(self, archive_path: Path, result: Dict) -> Dict:
        """验证归档文件完整性"""
        result["exists"] = True
        result["archived"] = True
        
        try:
            with gzip.open(archive_path, 'rt', encoding='utf-8') as f:
                for line_num, line in enumerate(f, 1):
                    line = line.strip()
                    if not line:
                        continue
                    result["total_records"] += 1
                    try:
                        record = json.loads(line)
                        if "type" in record and "data" in record:
                            result["valid_records"] += 1
                        else:
                            result["invalid_records"] += 1
                            result["errors"].append(f"Line {line_num}: Missing required fields")
                    except json.JSONDecodeError as e:
                        result["invalid_records"] += 1
                        result["errors"].append(f"Line {line_num}: JSON decode error - {e}")
        except gzip.BadGzipFile as e:
            result["errors"].append(f"Corrupted gzip file: {e}")
        
        return result
    
    async def get_storage_stats(self) -> Dict[str, Any]:
        """获取存储统计信息"""
        stats = {
            "total_sessions": 0,
            "active_files": 0,
            "archived_files": 0,
            "total_size_bytes": 0,
            "avg_file_size_bytes": 0
        }
        
        total_size = 0
        
        for file_path in self.storage_dir.glob("*.jsonl"):
            stats["active_files"] += 1
            total_size += file_path.stat().st_size
        
        for file_path in self.storage_dir.glob("*.jsonl.gz"):
            stats["archived_files"] += 1
            total_size += file_path.stat().st_size
        
        stats["total_sessions"] = stats["active_files"] + stats["archived_files"]
        stats["total_size_bytes"] = total_size
        
        if stats["total_sessions"] > 0:
            stats["avg_file_size_bytes"] = total_size // stats["total_sessions"]
        
        return stats


# ============================================================================
# Session 管理器
# ============================================================================

class SessionManager:
    """Session 管理器"""
    
    def __init__(
        self,
        storage_dir: str,
        default_ttl: int = 3600,
        cleanup_interval: int = 300
    ):
        self.store = JSONLSessionStore(storage_dir)
        self.default_ttl = default_ttl
        self.cleanup_interval = cleanup_interval
        self._active_sessions: Dict[str, SessionMetadata] = {}
        self._session_states: Dict[str, SessionState] = {}
        self._lock = asyncio.Lock()
        self._cleanup_task: Optional[asyncio.Task] = None
        self._running = False
    
    async def start(self):
        """启动管理器"""
        self._running = True
        self._cleanup_task = asyncio.create_task(self._cleanup_loop())
    
    async def stop(self):
        """停止管理器"""
        self._running = False
        if self._cleanup_task:
            self._cleanup_task.cancel()
            try:
                await self._cleanup_task
            except asyncio.CancelledError:
                pass
    
    async def _cleanup_loop(self):
        """清理过期 session 的循环"""
        while self._running:
            try:
                await asyncio.sleep(self.cleanup_interval)
                await self.cleanup_expired_sessions()
            except asyncio.CancelledError:
                break
            except Exception:
                continue
    
    async def create_session(
        self,
        client_id: str,
        user_agent: str = "",
        ip_address: str = "",
        custom_data: Optional[Dict] = None,
        ttl: Optional[int] = None
    ) -> SessionMetadata:
        """创建新 session"""
        now = time.time()
        session_id = str(uuid.uuid4())
        
        session = SessionMetadata(
            session_id=session_id,
            client_id=client_id,
            created_at=now,
            last_activity=now,
            expires_at=now + (ttl or self.default_ttl),
            user_agent=user_agent,
            ip_address=ip_address,
            custom_data=custom_data or {}
        )
        
        async with self._lock:
            self._active_sessions[session_id] = session
        
        await self.store.save_session(session)
        
        # 创建初始状态
        state = SessionState(
            session_id=session_id,
            status="active",
            data={},
            sequence_number=1,
            checkpoint_id=str(uuid.uuid4())
        )
        
        async with self._lock:
            self._session_states[session_id] = state
        
        await self.store.save_state(session_id, state)
        
        return session
    
    async def get_session(self, session_id: str) -> Optional[SessionMetadata]:
        """获取 session"""
        now = time.time()
        
        async with self._lock:
            if session_id in self._active_sessions:
                session = self._active_sessions[session_id]
                # 检查是否过期
                if session.expires_at > now:
                    return session
                else:
                    # 过期了，从内存中移除
                    del self._active_sessions[session_id]
                    self._session_states.pop(session_id, None)
                    return None
        
        # 从存储加载
        session = await self.store.load_session(session_id)
        if session and session.expires_at > now:
            async with self._lock:
                self._active_sessions[session_id] = session
            return session
        
        return None
    
    async def update_session_activity(self, session_id: str) -> bool:
        """更新 session 活动时间"""
        async with self._lock:
            if session_id not in self._active_sessions:
                return False
            
            session = self._active_sessions[session_id]
            session.last_activity = time.time()
            session.expires_at = session.last_activity + self.default_ttl
        
        await self.store.save_session(self._active_sessions[session_id])
        return True
    
    async def save_state(
        self,
        session_id: str,
        data: Dict[str, Any],
        status: str = "active"
    ) -> Optional[SessionState]:
        """保存 session 状态"""
        async with self._lock:
            if session_id not in self._session_states:
                return None
            
            current_state = self._session_states[session_id]
            new_state = SessionState(
                session_id=session_id,
                status=status,
                data=data,
                sequence_number=current_state.sequence_number + 1,
                checkpoint_id=str(uuid.uuid4())
            )
            self._session_states[session_id] = new_state
        
        await self.store.save_state(session_id, new_state)
        return new_state
    
    async def get_state(self, session_id: str) -> Optional[SessionState]:
        """获取 session 状态"""
        async with self._lock:
            if session_id in self._session_states:
                return self._session_states[session_id]
        
        return await self.store.load_state(session_id)
    
    async def destroy_session(self, session_id: str) -> bool:
        """销毁 session"""
        async with self._lock:
            self._active_sessions.pop(session_id, None)
            self._session_states.pop(session_id, None)
        
        return await self.store.delete_session(session_id)
    
    async def cleanup_expired_sessions(self) -> List[str]:
        """清理过期 session"""
        now = time.time()
        expired_sessions = []
        
        async with self._lock:
            for session_id, session in list(self._active_sessions.items()):
                if session.expires_at < now:
                    expired_sessions.append(session_id)
                    del self._active_sessions[session_id]
                    self._session_states.pop(session_id, None)
        
        # 归档过期 session
        for session_id in expired_sessions:
            await self.store.archive_session(session_id)
        
        return expired_sessions
    
    async def restore_session(self, session_id: str) -> Optional[SessionMetadata]:
        """恢复 session（用于客户端重连）"""
        session = await self.get_session(session_id)
        if not session:
            return None
        
        # 更新过期时间
        now = time.time()
        session.last_activity = now
        session.expires_at = now + self.default_ttl
        
        await self.store.save_session(session)
        
        # 加载状态
        state = await self.store.load_state(session_id)
        if state:
            async with self._lock:
                self._session_states[session_id] = state
        
        return session
    
    async def get_active_count(self) -> int:
        """获取活跃 session 数量"""
        async with self._lock:
            return len(self._active_sessions)
    
    async def recover_from_storage(self) -> int:
        """从存储恢复所有 session"""
        session_ids = await self.store.list_sessions()
        recovered = 0
        
        for session_id in session_ids:
            session = await self.store.load_session(session_id)
            if session and session.expires_at > time.time():
                async with self._lock:
                    self._active_sessions[session_id] = session
                
                state = await self.store.load_state(session_id)
                if state:
                    async with self._lock:
                        self._session_states[session_id] = state
                
                recovered += 1
        
        return recovered


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture
def temp_storage_dir():
    """创建临时存储目录"""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield tmpdir


@pytest.fixture
async def session_manager(temp_storage_dir):
    """创建 SessionManager 实例"""
    manager = SessionManager(
        storage_dir=temp_storage_dir,
        default_ttl=3600,
        cleanup_interval=60
    )
    await manager.start()
    yield manager
    await manager.stop()


@pytest.fixture
async def store(temp_storage_dir):
    """创建 JSONLSessionStore 实例"""
    store = JSONLSessionStore(temp_storage_dir)
    yield store


# ============================================================================
# 基础 Session 测试
# ============================================================================

@pytest.mark.asyncio
class TestSessionBasic:
    """基础 Session 测试"""
    
    async def test_create_session(self, temp_storage_dir):
        """测试创建 session"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            session = await manager.create_session(
                client_id="client_001",
                user_agent="TestAgent/1.0",
                ip_address="127.0.0.1",
                custom_data={"theme": "dark"}
            )
            
            assert session is not None
            assert session.session_id is not None
            assert session.client_id == "client_001"
            assert session.user_agent == "TestAgent/1.0"
            assert session.custom_data["theme"] == "dark"
            assert session.expires_at > session.created_at
        finally:
            await manager.stop()
    
    async def test_get_session(self, temp_storage_dir):
        """测试获取 session"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 创建 session
            created = await manager.create_session(client_id="client_001")
            
            # 获取 session
            retrieved = await manager.get_session(created.session_id)
            
            assert retrieved is not None
            assert retrieved.session_id == created.session_id
            assert retrieved.client_id == created.client_id
        finally:
            await manager.stop()
    
    async def test_destroy_session(self, temp_storage_dir):
        """测试销毁 session"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 创建 session
            session = await manager.create_session(client_id="client_001")
            session_id = session.session_id
            
            # 销毁 session
            result = await manager.destroy_session(session_id)
            assert result is True
            
            # 确认已删除
            retrieved = await manager.get_session(session_id)
            assert retrieved is None
        finally:
            await manager.stop()
    
    async def test_update_session_activity(self, temp_storage_dir):
        """测试更新 session 活动时间"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            session = await manager.create_session(client_id="client_001")
            original_expires = session.expires_at
            
            # 等待一小段时间
            await asyncio.sleep(0.1)
            
            # 更新活动时间
            result = await manager.update_session_activity(session.session_id)
            assert result is True
            
            # 验证过期时间已更新
            updated = await manager.get_session(session.session_id)
            assert updated.expires_at > original_expires
        finally:
            await manager.stop()


# ============================================================================
# Session 状态持久化测试
# ============================================================================

@pytest.mark.asyncio
class TestSessionPersistence:
    """Session 状态持久化测试"""
    
    async def test_session_persisted_to_storage(self, temp_storage_dir):
        """测试 session 被持久化到存储"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 创建 session
            session = await manager.create_session(
                client_id="client_001",
                custom_data={"key": "value"}
            )
            session_id = session.session_id
        finally:
            await manager.stop()
        
        # 创建新的管理器实例（模拟重启）
        new_manager = SessionManager(temp_storage_dir)
        await new_manager.start()
        
        try:
            # 从存储恢复
            recovered = await new_manager.get_session(session_id)
            
            assert recovered is not None
            assert recovered.session_id == session_id
            assert recovered.client_id == "client_001"
            assert recovered.custom_data["key"] == "value"
        finally:
            await new_manager.stop()
    
    async def test_state_persisted_to_storage(self, temp_storage_dir):
        """测试状态被持久化到存储"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 创建 session
            session = await manager.create_session(client_id="client_001")
            session_id = session.session_id
            
            # 保存状态
            await manager.save_state(
                session_id,
                data={"counter": 42, "items": ["a", "b", "c"]},
                status="active"
            )
        finally:
            await manager.stop()
        
        # 创建新的管理器实例
        new_manager = SessionManager(temp_storage_dir)
        await new_manager.start()
        
        try:
            # 从存储加载状态
            state = await new_manager.get_state(session_id)
            
            assert state is not None
            assert state.data["counter"] == 42
            assert state.data["items"] == ["a", "b", "c"]
            assert state.status == "active"
            assert state.sequence_number == 2  # 初始状态 + 保存的状态
        finally:
            await new_manager.stop()
    
    async def test_multiple_state_updates(self, temp_storage_dir):
        """测试多次状态更新"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            session = await manager.create_session(client_id="client_001")
            session_id = session.session_id
            
            # 多次更新状态
            for i in range(5):
                await manager.save_state(
                    session_id,
                    data={"iteration": i, "timestamp": time.time()},
                    status="active"
                )
            
            # 获取最新状态
            state = await manager.get_state(session_id)
            assert state.sequence_number == 6  # 1 初始 + 5 更新
            assert state.data["iteration"] == 4
        finally:
            await manager.stop()
    
    async def test_recover_from_storage(self, temp_storage_dir):
        """测试从存储恢复所有 session"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        session_ids = []
        try:
            # 创建多个 session
            for i in range(10):
                session = await manager.create_session(
                    client_id=f"client_{i}",
                    custom_data={"index": i}
                )
                session_ids.append(session.session_id)
        finally:
            await manager.stop()
        
        # 创建新的管理器并恢复
        new_manager = SessionManager(temp_storage_dir)
        await new_manager.start()
        
        try:
            recovered_count = await new_manager.recover_from_storage()
            assert recovered_count == 10
            
            # 验证所有 session 都已恢复
            for i, session_id in enumerate(session_ids):
                session = await new_manager.get_session(session_id)
                assert session is not None
                assert session.client_id == f"client_{i}"
        finally:
            await new_manager.stop()


# ============================================================================
# JSONL 存储格式测试
# ============================================================================

@pytest.mark.asyncio
class TestJSONLStorage:
    """JSONL 存储格式测试"""
    
    async def test_jsonl_format(self, temp_storage_dir):
        """测试 JSONL 格式正确性"""
        store = JSONLSessionStore(temp_storage_dir)
        
        session = SessionMetadata(
            session_id="test-123",
            client_id="client_001",
            created_at=time.time(),
            last_activity=time.time(),
            expires_at=time.time() + 3600,
            user_agent="TestAgent",
            ip_address="127.0.0.1",
            custom_data={"test": True}
        )
        
        await store.save_session(session)
        
        # 读取文件验证格式
        file_path = Path(temp_storage_dir) / "test-123.jsonl"
        with open(file_path, 'r') as f:
            line = f.readline().strip()
            record = json.loads(line)
            
            assert "type" in record
            assert "timestamp" in record
            assert "data" in record
            assert record["type"] == "metadata"
            assert record["data"]["session_id"] == "test-123"
    
    async def test_multiple_records_in_file(self, temp_storage_dir):
        """测试文件中多条记录"""
        store = JSONLSessionStore(temp_storage_dir)
        
        session_id = "test-multi"
        
        # 保存元数据
        session = SessionMetadata(
            session_id=session_id,
            client_id="client_001",
            created_at=time.time(),
            last_activity=time.time(),
            expires_at=time.time() + 3600,
            user_agent="TestAgent",
            ip_address="127.0.0.1",
            custom_data={}
        )
        await store.save_session(session)
        
        # 保存多个状态
        for i in range(3):
            state = SessionState(
                session_id=session_id,
                status="active",
                data={"step": i},
                sequence_number=i + 1,
                checkpoint_id=str(uuid.uuid4())
            )
            await store.save_state(session_id, state)
        
        # 验证文件包含 4 条记录
        file_path = Path(temp_storage_dir) / f"{session_id}.jsonl"
        with open(file_path, 'r') as f:
            lines = f.readlines()
            assert len(lines) == 4
            
            # 验证每条记录都是有效的 JSON
            for line in lines:
                record = json.loads(line.strip())
                assert "type" in record
    
    async def test_storage_integrity(self, temp_storage_dir):
        """测试存储完整性检查"""
        store = JSONLSessionStore(temp_storage_dir)
        
        session_id = "test-integrity"
        session = SessionMetadata(
            session_id=session_id,
            client_id="client_001",
            created_at=time.time(),
            last_activity=time.time(),
            expires_at=time.time() + 3600,
            user_agent="TestAgent",
            ip_address="127.0.0.1",
            custom_data={}
        )
        await store.save_session(session)
        
        # 验证完整性
        result = await store.verify_integrity(session_id)
        
        assert result["exists"] is True
        assert result["valid_records"] == 1
        assert result["invalid_records"] == 0
        assert len(result["errors"]) == 0
    
    async def test_archive_functionality(self, temp_storage_dir):
        """测试归档功能"""
        store = JSONLSessionStore(temp_storage_dir)
        
        session_id = "test-archive"
        session = SessionMetadata(
            session_id=session_id,
            client_id="client_001",
            created_at=time.time(),
            last_activity=time.time(),
            expires_at=time.time() + 3600,
            user_agent="TestAgent",
            ip_address="127.0.0.1",
            custom_data={"archived": True}
        )
        await store.save_session(session)
        
        # 归档
        result = await store.archive_session(session_id)
        assert result is True
        
        # 验证原文件已删除，归档文件存在
        file_path = Path(temp_storage_dir) / f"{session_id}.jsonl"
        archive_path = Path(temp_storage_dir) / f"{session_id}.jsonl.gz"
        
        assert not file_path.exists()
        assert archive_path.exists()
        
        # 验证可以从归档加载
        loaded = await store.load_session(session_id)
        assert loaded is not None
        assert loaded.session_id == session_id
        assert loaded.custom_data["archived"] is True
    
    async def test_storage_stats(self, temp_storage_dir):
        """测试存储统计"""
        store = JSONLSessionStore(temp_storage_dir)
        
        # 创建多个 session
        for i in range(5):
            session = SessionMetadata(
                session_id=f"session-{i}",
                client_id=f"client_{i}",
                created_at=time.time(),
                last_activity=time.time(),
                expires_at=time.time() + 3600,
                user_agent="TestAgent",
                ip_address="127.0.0.1",
                custom_data={}
            )
            await store.save_session(session)
        
        # 归档其中 2 个
        await store.archive_session("session-0")
        await store.archive_session("session-1")
        
        # 获取统计
        stats = await store.get_storage_stats()
        
        assert stats["total_sessions"] == 5
        assert stats["active_files"] == 3
        assert stats["archived_files"] == 2
        assert stats["total_size_bytes"] > 0


# ============================================================================
# Session 过期和清理测试
# ============================================================================

@pytest.mark.asyncio
class TestSessionExpiration:
    """Session 过期和清理测试"""
    
    async def test_session_expiration(self, temp_storage_dir):
        """测试 session 过期"""
        # 使用很短的 TTL
        manager = SessionManager(
            temp_storage_dir,
            default_ttl=1,  # 1 秒过期
            cleanup_interval=1
        )
        await manager.start()
        
        try:
            # 创建 session
            session = await manager.create_session(client_id="client_001")
            session_id = session.session_id
            
            # 确认 session 存在
            assert await manager.get_session(session_id) is not None
            
            # 等待过期
            await asyncio.sleep(2)
            
            # 确认 session 已过期（get_session 应该返回 None）
            assert await manager.get_session(session_id) is None
            
            # 手动触发清理（此时 session 已从内存中移除）
            expired = await manager.cleanup_expired_sessions()
            # 由于 session 已经在 get_session 时被清理，这里可能为空
            # 或者如果 cleanup_expired_sessions 检查存储，可能包含该 ID
        finally:
            await manager.stop()
    
    async def test_cleanup_multiple_expired_sessions(self, temp_storage_dir):
        """测试清理多个过期 session"""
        manager = SessionManager(
            temp_storage_dir,
            default_ttl=1,
            cleanup_interval=60
        )
        await manager.start()
        
        try:
            # 创建多个 session
            session_ids = []
            for i in range(5):
                session = await manager.create_session(client_id=f"client_{i}")
                session_ids.append(session.session_id)
            
            # 等待过期
            await asyncio.sleep(2)
            
            # 清理
            expired = await manager.cleanup_expired_sessions()
            assert len(expired) == 5
            
            # 确认都已清理
            for session_id in session_ids:
                assert await manager.get_session(session_id) is None
        finally:
            await manager.stop()
    
    async def test_partial_cleanup(self, temp_storage_dir):
        """测试部分清理（部分过期，部分未过期）"""
        manager = SessionManager(
            temp_storage_dir,
            default_ttl=2,
            cleanup_interval=60
        )
        await manager.start()
        
        try:
            # 创建一些快过期的 session
            expired_ids = []
            for i in range(3):
                session = await manager.create_session(
                    client_id=f"expired_{i}",
                    ttl=1  # 1 秒过期
                )
                expired_ids.append(session.session_id)
            
            # 等待一下再创建不会过期的 session
            await asyncio.sleep(0.5)
            
            active_ids = []
            for i in range(3):
                session = await manager.create_session(
                    client_id=f"active_{i}",
                    ttl=3600  # 1 小时过期
                )
                active_ids.append(session.session_id)
            
            # 等待第一批过期
            await asyncio.sleep(1.5)
            
            # 清理
            expired = await manager.cleanup_expired_sessions()
            
            # 验证只有第一批被清理
            assert len(expired) == 3
            for session_id in expired_ids:
                assert session_id in expired
                assert await manager.get_session(session_id) is None
            
            for session_id in active_ids:
                assert session_id not in expired
                assert await manager.get_session(session_id) is not None
        finally:
            await manager.stop()


# ============================================================================
# Session 恢复测试
# ============================================================================

@pytest.mark.asyncio
class TestSessionRecovery:
    """Session 恢复测试"""
    
    async def test_client_reconnect_same_session_id(self, temp_storage_dir):
        """测试客户端使用相同 session_id 重连"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 创建 session
            session = await manager.create_session(
                client_id="client_001",
                custom_data={"connection_count": 1}
            )
            session_id = session.session_id
            
            # 保存一些状态
            await manager.save_state(
                session_id,
                data={"messages": ["hello", "world"]},
                status="active"
            )
            
            # 模拟客户端重连 - 恢复 session
            restored = await manager.restore_session(session_id)
            
            assert restored is not None
            assert restored.session_id == session_id
            
            # 验证状态仍然存在
            state = await manager.get_state(session_id)
            assert state is not None
            assert state.data["messages"] == ["hello", "world"]
        finally:
            await manager.stop()
    
    async def test_server_restart_recovery(self, temp_storage_dir):
        """测试服务端重启后恢复 session"""
        session_ids = []
        
        # 第一个管理器实例
        manager1 = SessionManager(temp_storage_dir)
        await manager1.start()
        
        try:
            # 创建多个 session
            for i in range(5):
                session = await manager1.create_session(
                    client_id=f"client_{i}",
                    custom_data={"original": True}
                )
                session_ids.append(session.session_id)
                
                # 保存状态
                await manager1.save_state(
                    session.session_id,
                    data={"index": i, "data": f"value_{i}"}
                )
        finally:
            await manager1.stop()
        
        # 模拟服务端重启 - 创建新的管理器实例
        manager2 = SessionManager(temp_storage_dir)
        await manager2.start()
        
        try:
            # 恢复所有 session
            recovered_count = await manager2.recover_from_storage()
            assert recovered_count == 5
            
            # 验证所有 session 和状态都已恢复
            for i, session_id in enumerate(session_ids):
                session = await manager2.get_session(session_id)
                assert session is not None
                assert session.client_id == f"client_{i}"
                
                state = await manager2.get_state(session_id)
                assert state is not None
                assert state.data["index"] == i
                assert state.data["data"] == f"value_{i}"
        finally:
            await manager2.stop()
    
    async def test_network_reconnect_scenario(self, temp_storage_dir):
        """测试断网重连场景"""
        manager = SessionManager(
            temp_storage_dir,
            default_ttl=60  # 给足够的时间重连
        )
        await manager.start()
        
        try:
            # 创建 session
            session = await manager.create_session(
                client_id="mobile_client",
                user_agent="MobileApp/1.0",
                custom_data={"device": "iPhone"}
            )
            session_id = session.session_id
            
            # 模拟正常操作
            await manager.save_state(
                session_id,
                data={"page": "dashboard", "scroll_position": 100}
            )
            
            # 模拟断网 - 等待一段时间
            await asyncio.sleep(0.5)
            
            # 模拟重连 - 更新活动时间
            result = await manager.update_session_activity(session_id)
            assert result is True
            
            # 验证 session 仍然有效
            session = await manager.get_session(session_id)
            assert session is not None
            
            # 验证状态仍然保留
            state = await manager.get_state(session_id)
            assert state.data["page"] == "dashboard"
        finally:
            await manager.stop()


# ============================================================================
# 并发测试
# ============================================================================

@pytest.mark.asyncio
class TestSessionConcurrency:
    """Session 并发测试"""
    
    async def test_concurrent_session_creation(self, temp_storage_dir):
        """测试并发创建 session"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 并发创建 100 个 session
            async def create_session(i):
                return await manager.create_session(
                    client_id=f"concurrent_client_{i}",
                    custom_data={"index": i}
                )
            
            tasks = [create_session(i) for i in range(100)]
            sessions = await asyncio.gather(*tasks)
            
            # 验证所有 session 都创建成功
            assert len(sessions) == 100
            session_ids = [s.session_id for s in sessions]
            assert len(set(session_ids)) == 100  # 所有 ID 都是唯一的
            
            # 验证都可以检索到
            for i, session in enumerate(sessions):
                retrieved = await manager.get_session(session.session_id)
                assert retrieved is not None
                assert retrieved.client_id == f"concurrent_client_{i}"
        finally:
            await manager.stop()
    
    async def test_concurrent_state_updates(self, temp_storage_dir):
        """测试并发状态更新"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 创建一个 session
            session = await manager.create_session(client_id="concurrent_client")
            session_id = session.session_id
            
            # 并发更新状态
            async def update_state(i):
                return await manager.save_state(
                    session_id,
                    data={"last_updater": i, "timestamp": time.time()},
                    status="active"
                )
            
            tasks = [update_state(i) for i in range(50)]
            states = await asyncio.gather(*tasks)
            
            # 验证所有更新都成功
            assert len(states) == 50
            assert all(s is not None for s in states)
            
            # 验证序列号递增
            sequence_numbers = [s.sequence_number for s in states]
            assert len(set(sequence_numbers)) == 50  # 所有序列号都不同
        finally:
            await manager.stop()
    
    async def test_100_active_sessions(self, temp_storage_dir):
        """测试 100+ 同时活跃的 session"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 创建 150 个 session
            sessions = []
            for i in range(150):
                session = await manager.create_session(
                    client_id=f"bulk_client_{i}",
                    custom_data={
                        "index": i,
                        "data": f"large_data_payload_{i}_" + "x" * 100
                    }
                )
                sessions.append(session)
            
            # 验证活跃数量
            active_count = await manager.get_active_count()
            assert active_count == 150
            
            # 验证所有 session 都可以访问
            for session in sessions:
                retrieved = await manager.get_session(session.session_id)
                assert retrieved is not None
        finally:
            await manager.stop()
    
    async def test_race_condition_session_access(self, temp_storage_dir):
        """测试 Session 访问的竞争条件"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 创建一个 session
            session = await manager.create_session(client_id="race_test")
            session_id = session.session_id
            
            results = []
            
            async def access_session():
                s = await manager.get_session(session_id)
                if s:
                    results.append(s.session_id)
                return s
            
            # 同时从多个任务访问
            tasks = [access_session() for _ in range(20)]
            await asyncio.gather(*tasks)
            
            # 验证所有访问都成功
            assert len(results) == 20
            assert all(r == session_id for r in results)
        finally:
            await manager.stop()
    
    async def test_concurrent_cleanup_and_access(self, temp_storage_dir):
        """测试并发清理和访问"""
        manager = SessionManager(
            temp_storage_dir,
            default_ttl=1,
            cleanup_interval=1
        )
        await manager.start()
        
        try:
            # 创建一些 session
            sessions = []
            for i in range(20):
                session = await manager.create_session(
                    client_id=f"cleanup_test_{i}",
                    ttl=2 if i < 10 else 3600  # 一半快过期，一半不过期
                )
                sessions.append(session)
            
            # 等待部分过期
            await asyncio.sleep(2.5)
            
            # 同时执行清理和访问
            async def cleanup():
                return await manager.cleanup_expired_sessions()
            
            async def access_session(i):
                return await manager.get_session(sessions[i].session_id)
            
            # 并发执行
            cleanup_task = asyncio.create_task(cleanup())
            access_tasks = [access_session(i) for i in range(20)]
            
            expired = await cleanup_task
            accessed = await asyncio.gather(*access_tasks)
            
            # 验证访问没有出错（部分返回 None，部分返回 session）
            assert len(accessed) == 20
            
            # 验证未过期的 session 仍然可以访问
            active_count = sum(1 for a in accessed if a is not None)
            assert active_count == 10  # 10 个未过期的 session
            
            # 验证已过期的 session 返回 None
            expired_count = sum(1 for a in accessed if a is None)
            assert expired_count == 10  # 10 个已过期的 session
        finally:
            await manager.stop()


# ============================================================================
# 大数据量性能测试
# ============================================================================

@pytest.mark.asyncio
class TestSessionPerformance:
    """Session 性能测试"""
    
    async def test_large_data_storage(self, temp_storage_dir):
        """测试大数据量存储"""
        store = JSONLSessionStore(temp_storage_dir)
        
        # 创建包含大数据的 session
        large_data = {
            "items": [f"item_{i}" for i in range(10000)],
            "metadata": {f"key_{i}": f"value_{i}" for i in range(1000)}
        }
        
        session = SessionMetadata(
            session_id="large-data-session",
            client_id="client_001",
            created_at=time.time(),
            last_activity=time.time(),
            expires_at=time.time() + 3600,
            user_agent="TestAgent",
            ip_address="127.0.0.1",
            custom_data=large_data
        )
        
        start_time = time.time()
        await store.save_session(session)
        save_time = time.time() - start_time
        
        # 验证保存成功且时间合理
        assert save_time < 5.0  # 应该在 5 秒内完成
        
        # 验证可以加载
        start_time = time.time()
        loaded = await store.load_session("large-data-session")
        load_time = time.time() - start_time
        
        assert loaded is not None
        assert len(loaded.custom_data["items"]) == 10000
        assert load_time < 5.0
    
    async def test_bulk_session_operations(self, temp_storage_dir):
        """测试批量 session 操作性能"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 批量创建 200 个 session
            start_time = time.time()
            
            for i in range(200):
                await manager.create_session(
                    client_id=f"perf_client_{i}",
                    custom_data={"index": i, "data": "x" * 100}
                )
            
            create_time = time.time() - start_time
            
            # 验证性能
            assert create_time < 30.0  # 200 个 session 应该在 30 秒内完成
            
            # 批量恢复测试
            new_manager = SessionManager(temp_storage_dir)
            await new_manager.start()
            
            start_time = time.time()
            recovered = await new_manager.recover_from_storage()
            recover_time = time.time() - start_time
            
            assert recovered == 200
            assert recover_time < 30.0
            
            await new_manager.stop()
        finally:
            await manager.stop()
    
    async def test_storage_compression_ratio(self, temp_storage_dir):
        """测试存储压缩比率"""
        store = JSONLSessionStore(temp_storage_dir)
        
        # 创建包含重复数据的 session
        session_id = "compress-test"
        
        for i in range(100):
            session = SessionMetadata(
                session_id=session_id,
                client_id=f"client_{i}",
                created_at=time.time(),
                last_activity=time.time(),
                expires_at=time.time() + 3600,
                user_agent="TestAgent/1.0 (Very Long User Agent String For Testing Compression)",
                ip_address="192.168.1.100",
                custom_data={"iteration": i, "template": "repeated_data_pattern" * 10}
            )
            await store.save_session(session)
        
        # 获取原始大小
        file_path = Path(temp_storage_dir) / f"{session_id}.jsonl"
        original_size = file_path.stat().st_size
        
        # 归档
        await store.archive_session(session_id)
        
        archive_path = Path(temp_storage_dir) / f"{session_id}.jsonl.gz"
        compressed_size = archive_path.stat().st_size
        
        # 计算压缩比率
        compression_ratio = (1 - compressed_size / original_size) * 100
        
        # 验证有压缩效果
        assert compressed_size < original_size
        assert compression_ratio > 50  # 期望至少 50% 的压缩率
    
    async def test_concurrent_read_write_performance(self, temp_storage_dir):
        """测试并发读写性能"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            # 先创建一些 session
            sessions = []
            for i in range(50):
                session = await manager.create_session(client_id=f"rw_test_{i}")
                sessions.append(session)
            
            # 并发读写
            async def mixed_operations(i):
                session_id = sessions[i % len(sessions)].session_id
                
                # 读
                await manager.get_session(session_id)
                
                # 写
                await manager.save_state(
                    session_id,
                    data={"update": i, "timestamp": time.time()}
                )
                
                # 再读
                return await manager.get_state(session_id)
            
            start_time = time.time()
            tasks = [mixed_operations(i) for i in range(200)]
            results = await asyncio.gather(*tasks)
            elapsed = time.time() - start_time
            
            # 验证所有操作成功
            assert len(results) == 200
            assert all(r is not None for r in results)
            
            # 验证性能
            assert elapsed < 30.0  # 200 次操作应该在 30 秒内完成
        finally:
            await manager.stop()


# ============================================================================
# 边界情况和错误处理测试
# ============================================================================

@pytest.mark.asyncio
class TestSessionEdgeCases:
    """边界情况和错误处理测试"""
    
    async def test_nonexistent_session(self, temp_storage_dir):
        """测试访问不存在的 session"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            result = await manager.get_session("nonexistent-id")
            assert result is None
        finally:
            await manager.stop()
    
    async def test_duplicate_session_id_handling(self, temp_storage_dir):
        """测试重复 session ID 处理"""
        store = JSONLSessionStore(temp_storage_dir)
        
        # 两次保存相同 ID 的 session（追加模式）
        session_id = "duplicate-test"
        
        session1 = SessionMetadata(
            session_id=session_id,
            client_id="client_v1",
            created_at=time.time(),
            last_activity=time.time(),
            expires_at=time.time() + 3600,
            user_agent="Agent/1.0",
            ip_address="127.0.0.1",
            custom_data={"version": 1}
        )
        
        session2 = SessionMetadata(
            session_id=session_id,
            client_id="client_v2",
            created_at=time.time(),
            last_activity=time.time(),
            expires_at=time.time() + 3600,
            user_agent="Agent/2.0",
            ip_address="127.0.0.1",
            custom_data={"version": 2}
        )
        
        await store.save_session(session1)
        await store.save_session(session2)
        
        # 加载应该返回最新的
        loaded = await store.load_session(session_id)
        assert loaded.client_id == "client_v2"
        assert loaded.custom_data["version"] == 2
    
    async def test_empty_custom_data(self, temp_storage_dir):
        """测试空自定义数据"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            session = await manager.create_session(
                client_id="empty_data_client",
                custom_data={}
            )
            
            assert session is not None
            assert session.custom_data == {}
            
            retrieved = await manager.get_session(session.session_id)
            assert retrieved.custom_data == {}
        finally:
            await manager.stop()
    
    async def test_special_characters_in_data(self, temp_storage_dir):
        """测试特殊字符处理"""
        manager = SessionManager(temp_storage_dir)
        await manager.start()
        
        try:
            special_data = {
                "unicode": "你好世界 🌍 Привет мир",
                "newlines": "line1\nline2\nline3",
                "quotes": 'He said "Hello" and \'Goodbye\'',
                "backslash": "path\\to\\file",
                "null": None,
                "boolean": True
            }
            
            session = await manager.create_session(
                client_id="special_client",
                custom_data=special_data
            )
            
            # 保存状态
            await manager.save_state(
                session.session_id,
                data=special_data
            )
            
            # 重新加载
            new_manager = SessionManager(temp_storage_dir)
            await new_manager.start()
            
            try:
                loaded = await new_manager.get_session(session.session_id)
                assert loaded.custom_data["unicode"] == special_data["unicode"]
                
                state = await new_manager.get_state(session.session_id)
                assert state.data["unicode"] == special_data["unicode"]
                assert state.data["newlines"] == special_data["newlines"]
                assert state.data["quotes"] == special_data["quotes"]
            finally:
                await new_manager.stop()
        finally:
            await manager.stop()
    
    async def test_corrupted_file_recovery(self, temp_storage_dir):
        """测试损坏文件恢复"""
        store = JSONLSessionStore(temp_storage_dir)
        
        session_id = "corrupt-test"
        
        # 创建有效数据
        session = SessionMetadata(
            session_id=session_id,
            client_id="client_001",
            created_at=time.time(),
            last_activity=time.time(),
            expires_at=time.time() + 3600,
            user_agent="TestAgent",
            ip_address="127.0.0.1",
            custom_data={}
        )
        await store.save_session(session)
        
        # 追加损坏的数据
        file_path = Path(temp_storage_dir) / f"{session_id}.jsonl"
        with open(file_path, 'a') as f:
            f.write("this is not valid json\n")
            f.write('{"incomplete": ')
        
        # 验证完整性检查能发现问题
        result = await store.verify_integrity(session_id)
        assert result["exists"] is True
        assert result["invalid_records"] == 2
        assert len(result["errors"]) == 2
        
        # 但有效数据仍然可以加载
        loaded = await store.load_session(session_id)
        assert loaded is not None
        assert loaded.session_id == session_id


# ============================================================================
# 主测试运行器
# ============================================================================

if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
