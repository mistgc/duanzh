// DUANZH - Text Chapterizer Frontend Script

document.addEventListener('DOMContentLoaded', function() {
    const fileInput = document.getElementById('fileInput');
    const browseBtn = document.getElementById('browseBtn');
    const uploadArea = document.getElementById('uploadArea');
    const progressSection = document.getElementById('progressSection');
    const progressFill = document.getElementById('progressFill');
    const progressText = document.getElementById('progressText');
    const resultSection = document.getElementById('resultSection');
    const resultText = document.getElementById('resultText');
    const chapterCount = document.getElementById('chapterCount');
    const downloadBtn = document.getElementById('downloadBtn');
    
    // Click on upload area to trigger file input
    uploadArea.addEventListener('click', function() {
        if (!resultSection.style.display || resultSection.style.display === 'none') {
            fileInput.click();
        }
    });
    
    // Click on browse button to trigger file input
    browseBtn.addEventListener('click', function() {
        fileInput.click();
    });
    
    // Handle file selection
    fileInput.addEventListener('change', function(e) {
        if (e.target.files.length > 0) {
            const file = e.target.files[0];
            processFile(file);
        }
    });
    
    // Process file function
    async function processFile(file) {
        // Validate file type
        if (!file.name.toLowerCase().endsWith('.txt')) {
            alert('Please select a .txt file');
            return;
        }
        
        // Show progress section
        uploadArea.style.display = 'none';
        progressSection.style.display = 'block';
        resultSection.style.display = 'none';
        
        // Show initial progress
        progressFill.style.width = '20%';
        progressText.textContent = '正在上传文件...';
        
        try {
            // Create FormData for file upload
            const formData = new FormData();
            formData.append('text_file', file);
            
            // Update progress to 50% during upload
            progressFill.style.width = '50%';
            progressText.textContent = '上传完成，正在处理...';
            
            // Send the file to the backend
            const response = await fetch('/upload', {
                method: 'POST',
                body: formData
            });
            
            if (!response.ok) {
                throw new Error('服务器响应错误: ' + response.status);
            }
            
            const data = await response.json();
            
            // Update progress to 90% when processing complete
            progressFill.style.width = '90%';
            progressText.textContent = '处理完成，准备下载...';
            
            // Update result section
            if (data.success) {
                chapterCount.textContent = `${data.chapter_count} 章节`;
                
                // Update download button with the actual download URL
                if (data.download_url) {
                    downloadBtn.href = data.download_url;
                }
                
                // Show result section
                setTimeout(() => {
                    progressFill.style.width = '100%';
                    progressText.textContent = '处理完成！';
                    
                    setTimeout(() => {
                        progressSection.style.display = 'none';
                        resultSection.style.display = 'block';
                    }, 500);
                }, 500);
            } else {
                throw new Error('处理失败');
            }
        } catch (error) {
            console.error('Error processing file:', error);
            progressSection.style.display = 'none';
            uploadArea.style.display = 'block';
            alert('文件处理失败: ' + error.message);
        }
    }
    
    // Drag and drop functionality
    uploadArea.addEventListener('dragover', function(e) {
        e.preventDefault();
        uploadArea.style.borderColor = '#4f46e5';
        uploadArea.style.backgroundColor = '#eff6ff';
    });
    
    uploadArea.addEventListener('dragleave', function() {
        uploadArea.style.borderColor = '#cbd5e1';
        uploadArea.style.backgroundColor = '#f8fafc';
    });
    
    uploadArea.addEventListener('drop', function(e) {
        e.preventDefault();
        uploadArea.style.borderColor = '#cbd5e1';
        uploadArea.style.backgroundColor = '#f8fafc';
        
        if (e.dataTransfer.files.length > 0) {
            const file = e.dataTransfer.files[0];
            if (file.type === 'text/plain' || file.name.toLowerCase().endsWith('.txt')) {
                processFile(file);
            } else {
                alert('请上传 .txt 文件');
            }
        }
    });
});

