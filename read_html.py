import re
import sys

def extract_text(html_file):
    try:
        with open(html_file, 'r', encoding='utf-8') as f:
            content = f.read()
            methods = re.findall(r'<h4[^>]*class="method"[^>]*>(.*?)</h4>', content, re.DOTALL)
            for m in methods:
                text = re.sub(r'<[^>]*>', '', m).strip()
                # Remove \n and excessive spaces
                text = ' '.join(text.split())
                print(text)
    except Exception as e:
        print(f"Error: {e}")

if __name__ == '__main__':
    extract_text(sys.argv[1])
